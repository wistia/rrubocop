import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from types import SimpleNamespace
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from parity_audit.common import identity
from parity_audit.compare import Comparison
from parity_audit.literals import generate, merge, selected_seeds

PROJECT = os.environ.get("AUDIT_RUBY_PROJECT")
ROOT = Path(__file__).resolve().parents[3]
COMMENT = "\n# parity audit\n"


def seed(source, cop="Style/Example", **extra):
    return {"cop": cop, "source": source, "path": "probe.rb", "config": {}, "origin": "reference", **extra}


class MutationBudgetTest(unittest.TestCase):
    def test_seed_budget_is_per_cop_and_does_not_authorize_runtime(self):
        cases = [seed(f"foo({i})\n") for i in range(5)]
        cases += [seed("foo(9)\n", runtime={})]
        self.assertEqual([row["source"] for row in selected_seeds(cases, 2)], ["foo(0)\n", "foo(1)\n"])

    def test_existing_identity_and_runtime_authorization_survive_merge(self):
        original = {**seed("foo(1)\n"), "runtime": {"watch": ["sample"]}}
        ordinary = seed("foo(1)\n")
        a = merge([original, ordinary], [])
        b = merge(a, [seed("foo(0x1)\n")])
        self.assertEqual([(identity(row), row["id"]) for row in a],
                         [(identity(row), row["id"]) for row in b[:2]])
        self.assertNotIn("runtime", b[-1])

    def test_invalid_generated_originals_are_not_passing_examples(self):
        case = {**seed('invalid generated syntax'), 'id': 'invalid', 'origin': 'source-encoding:UTF-8'}
        native = {(case['id'], mode): {"source": case["source"], "diagnostics": []}
                  for mode in ("off", "safe", "all")}
        rows = Comparison(case, native, {"original_invalid": True, "original_compile": {"valid": False}}).run()
        self.assertTrue(any(row["status"] == "invalid_mutation" for row in rows))
        self.assertFalse(any(row["status"] == "pass" for row in rows))


@unittest.skipUnless(PROJECT, "set AUDIT_RUBY_PROJECT for Ruby generator integration checks")
class AstMutationTest(unittest.TestCase):
    def generate(self, source, **extra):
        args = SimpleNamespace(ruby_project=Path(PROJECT), timeout=60)
        with tempfile.TemporaryDirectory() as tmp:
            return generate([seed(source, **extra)], args, ROOT, Path(tmp))

    def inspect(self, sources):
        with tempfile.TemporaryDirectory() as tmp:
            input_path, output_path = Path(tmp) / "input.json", Path(tmp) / "output.json"
            input_path.write_text(json.dumps(sources))
            subprocess.run(["bundle", "exec", "ruby", str(ROOT / "scripts/parity_audit/tests/generator_probe.rb"),
                            str(input_path), str(output_path)], cwd=PROJECT, check=True, capture_output=True, text=True)
            return json.loads(output_path.read_text())

    def test_comments_cover_later_calls_operator_args_and_both_accessor_arguments(self):
        source = "setup(99); 1.5.eql?(0); obj.class.eql?(String); bag[0]; bag[1] = 2\n"
        rows = self.generate(source)
        sources = [row["source"] for row in rows]
        for snippet in [f"1.5.eql?({COMMENT}0)", f"obj.class.eql?({COMMENT}String)",
                        f"bag[{COMMENT}0]", f"bag[{COMMENT}1] = 2", f"bag[1] = {COMMENT}2"]:
            self.assertTrue(any(snippet in changed for changed in sources), snippet)
        comments = [row["source"] for row in rows if row["origin"].startswith("ast-")]
        self.assertTrue(all(row["compile"]["valid"] for row in self.inspect(comments)))

    def test_operator_calls_diversify_binary_seeds(self):
        rows = self.generate("obj.class == String\n")
        sources = [row["source"] for row in rows]
        for method in ["==", "eql?", "equal?"]:
            self.assertIn(f"obj.class.{method}({COMMENT}String)\n", sources)
        complex_rows = self.generate("(a + b) == 1\n")
        self.assertTrue(any(".==(" in row["source"] and "(a + b)" in row["source"] for row in complex_rows))

    def test_method_variants_change_only_one_identical_operator_occurrence(self):
        rows = self.generate("obj.class == String; obj.class == String\n")
        variants = [row for row in rows if row["origin"].startswith("operator-call-comment:")
                    and row["origin"].endswith(":eql?")]
        self.assertEqual(len(variants), 2)
        self.assertTrue(all(row["source"].count(".eql?(") == 1 for row in variants))
        self.assertTrue(all(row["source"].count("class == String") == 1 for row in variants))

    def test_command_call_and_operator_assignment_comments_preserve_ast(self):
        source = "receiver.add a, b; x += 1\n"
        rows = [row for row in self.generate(source) if row["origin"].startswith("ast-argument-comment")]
        self.assertGreaterEqual(len(rows), 3)
        properties = self.inspect([source] + [row["source"] for row in rows])
        self.assertTrue(all(row["ast"] == properties[0]["ast"] for row in properties))

    def test_empty_call_argument_comments_are_valid(self):
        rows = self.generate("setup(); obj.class.eql?(String)\n")
        self.assertTrue(any(f"obj.class({COMMENT})" in row["source"] for row in rows))
        self.assertTrue(any(f"setup({COMMENT})" in row["source"] for row in rows))

    def test_unpack_receiver_gets_a_real_attached_block(self):
        rows = self.generate("data.unpack('C*').first\n")
        matches = [row for row in rows if ".unpack('C*') { |__parity_value| __parity_value }.first" in row["source"]]
        self.assertEqual(len(matches), 1)
        self.assertTrue(self.inspect([matches[0]["source"]])[0]["compile"]["valid"])
        blocked = self.generate("data.unpack('C*', &handler).first\n")
        self.assertFalse(any("&handler) {" in row["source"] for row in blocked))

    def test_numeric_spellings_preserve_integer_and_float_values(self):
        source = "1.times { work }; 1.5.eql?(0); -0.0 == 12345\n"
        rows = [row for row in self.generate(source) if row["origin"].startswith(("integer-", "float-"))]
        self.assertTrue(any("0x1.times" in row["source"] for row in rows))
        self.assertTrue(any("1.50000000000000000e+00.eql?" in row["source"] for row in rows))
        properties = self.inspect([source] + [row["source"] for row in rows])
        self.assertTrue(all(row["numbers"] == properties[0]["numbers"] for row in properties))

    def test_nested_symbol_labels_and_binary_encoding_are_preserved(self):
        source = "# encoding: ASCII-8BIT\n[:é, {name: :value}]\n"
        rows = self.generate(source)
        nested = [row for row in rows if row["origin"] == "nested-encoded-literal"]
        self.assertGreaterEqual(len(nested), 3)
        properties = self.inspect([source] + [row["source"] for row in nested])
        self.assertTrue(all(row["symbols"] == properties[0]["symbols"] for row in properties))
        encodings = [row for row in rows if row["origin"].startswith("source-encoding:")]
        properties = self.inspect([row["source"] for row in encodings])
        self.assertEqual({row["symbols"][0]["encoding"] for row in properties}, {"UTF-8", "ASCII-8BIT"})

    def test_source_encoding_can_produce_invalid_mutations_without_executing_them(self):
        rows = self.generate('# encoding: ASCII-8BIT\n:"\\xff"\n')
        utf8 = next(row for row in rows if row["origin"] == "source-encoding:UTF-8")
        self.assertFalse(self.inspect([utf8["source"]])[0]["compile"]["valid"])

    def test_deterministic_mutations_never_inherit_runtime_authorization(self):
        source = 'raise "must not execute"; foo(1)\n'
        a = self.generate(source, runtime={"files": {"sample": "old"}})
        b = self.generate(source, runtime={"files": {"sample": "old"}})
        self.assertEqual(a, b)
        self.assertTrue(a)
        self.assertTrue(all("runtime" not in row for row in a))
