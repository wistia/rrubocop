from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from parity_audit.cases import references
from parity_audit.diagnostics import compare_diagnostics, compare_ranges
from parity_audit.outcomes import correction_result, syntax_results
from parity_audit.runtime import outcome
from parity_audit.compare import validate_results, Comparison
from parity_audit.policy import declared_diagnostics, declared_source


def case():
    return {"id": "test", "cop": "Style/Example", "source": "original", "path": "probe.rb",
            "origin": "reference", "expected": [], "corrected": "original", "config": {}}


def diagnostic(length=5, severity="convention"):
    return {"line": 1, "column": 0, "message": "message", "correctable": False,
            "length": length, "severity": severity}


class AuditClassifierTest(unittest.TestCase):
    def test_reason_cannot_hide_native_error_even_for_upstream_bad_rewrite(self):
        example = {**case(), "reason": "manual correction", "rust_corrected": "original"}
        rows = correction_result(example, "all", {"source": "original", "error": "internal error"},
                                 {"source": "broken", "compile": {"valid": False}})
        self.assertEqual(rows[0]["status"], "failure")

    def test_range_and_severity_are_not_covered_by_correctability_exception(self):
        example = {**case(), "reason": "manual", "expected": [diagnostic()], "rust_correctable": False}
        actual = {"diagnostics": [{"location": {"line": 1, "column": 0}, "message": "message",
                                   "correctable": False, "highlight_length": 500, "severity": "Error"}]}
        rows = compare_diagnostics(example, actual, {"diagnostics": [diagnostic()]})
        self.assertEqual([row["kind"] for row in rows], ["range_or_severity"])
        self.assertEqual(rows[0]["status"], "failure")

    def test_duplicate_locations_compare_range_multisets(self):
        rows = [diagnostic(10), diagnostic(20)]
        self.assertEqual(compare_ranges(case(), rows, list(reversed(rows))), [])

    def test_removed_duplicate_cannot_hide_unrelated_range_change(self):
        rows = compare_ranges(case(), [diagnostic(999)], [diagnostic(10), diagnostic(20)])
        self.assertEqual(rows[0]["status"], "failure")

    def test_distinct_runtime_scenarios_are_not_deduplicated(self):
        a = {**case(), "runtime": {"files": {"sample": "old"}}}
        b = {**case(), "runtime": {"files": {}}}
        self.assertEqual(len(references([a, b], None)), 2)

    def test_missing_worker_output_is_not_a_pass(self):
        with self.assertRaisesRegex(ValueError, "Incomplete"):
            validate_results([case()], {}, {})

    def test_runtime_native_change_and_upstream_change_are_distinct(self):
        self.assertEqual(outcome(case(), "all", {}, {"changed": True}, {})["status"], "failure")
        self.assertEqual(outcome(case(), "all", {}, {"changed": True}, {"changed": True})["status"], "upstream_hazard")

    def test_timeout_is_not_runtime_equivalence(self):
        failed = {"runner_error": "timeout"}
        self.assertEqual(outcome(case(), "all", failed, failed, failed)["status"], "failure")

    def test_compiler_catches_native_only_invalid_ruby(self):
        ruby = {"original_compile": {"valid": True}, "all": {"source": "valid", "compile": {"valid": True}},
                "native_compile": {"all": {"valid": False, "error": "bad regexp"}}}
        rows = syntax_results(case(), "all", ruby, {"source": "invalid"})
        self.assertEqual(rows[0]["kind"], "native_invalid_ruby")

    def test_inherited_invalid_ruby_is_separate(self):
        ruby = {"original_compile": {"valid": True}, "all": {"source": "invalid", "compile": {"valid": False}},
                "native_compile": {"all": {"valid": False, "error": "bad regexp"}}}
        rows = syntax_results(case(), "all", ruby, {"source": "invalid"})
        self.assertEqual([r["status"] for r in rows], ["upstream_hazard"])


class ExceptionBoundaryTest(unittest.TestCase):
    def test_source_reason_does_not_allow_metadata_drift(self):
        example = {**case(), "reason": "source safeguard", "rust_corrected": "original"}
        self.assertFalse(declared_diagnostics(example, []))

    def test_metadata_reason_does_not_allow_source_drift(self):
        example = {**case(), "reason": "diagnostic safeguard", "rust_correctable": False}
        self.assertFalse(declared_source(example, "all", "original"))

    def test_invalid_generated_source_does_not_hide_native_panic(self):
        example = {**case(), "origin": "parentheses"}
        native = {(example["id"], m): {"error": "native cop panicked"} for m in ("off", "safe", "all")}
        rows = Comparison(example, native, {"error": "Invalid Ruby syntax in case 1"}).run()
        self.assertEqual(sum(r["status"] == "failure" for r in rows), 3)
        self.assertEqual(sum(r["status"] == "invalid_mutation" for r in rows), 1)


class WorkerOutputTest(unittest.TestCase):
    def test_old_outputs_are_removed_even_if_worker_does_nothing(self):
        import tempfile
        from unittest.mock import patch
        from parity_audit.runner import engines
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp)
            for name in ("native.jsonl", "reference.jsonl", "report.md"):
                (output / name).write_text("old")
            with patch("parity_audit.runner.record"), patch("parity_audit.runner.native_engine"), patch("parity_audit.runner.ruby_engine"):
                engines(None, None, output)
            self.assertFalse((output / "native.jsonl").exists())
            self.assertFalse((output / "reference.jsonl").exists())
            self.assertFalse((output / "report.md").exists())

    def test_unrelated_output_directory_is_not_overwritten(self):
        import tempfile
        from custom_parity_audit import prepare_output
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp)
            report = output / "report.md"
            report.write_text("user report")
            with self.assertRaises(ValueError):
                prepare_output(output)
            self.assertEqual(report.read_text(), "user report")


class CoverageTest(unittest.TestCase):
    def test_all_invalid_examples_do_not_pass(self):
        from custom_parity_audit import exit_status
        self.assertEqual(exit_status({"compared_cases": 0}, None), 2)

    def test_invalid_examples_are_not_counted_as_compared(self):
        from parity_audit.report import comparison_coverage
        result = comparison_coverage([case()], [case()["cop"]],
                                     [{"id": "test", "kind": "original_invalid_ruby"}])
        self.assertEqual(result["compared_cases"], 0)
        self.assertEqual(result["uncompared_cops"], [case()["cop"]])
