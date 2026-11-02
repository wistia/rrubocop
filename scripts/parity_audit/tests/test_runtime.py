import base64
import os
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from parity_audit.runtime import execute

PROJECT = os.environ.get("AUDIT_RUBY_PROJECT")
ROOT = Path(__file__).resolve().parents[3]


@unittest.skipUnless(PROJECT, "set AUDIT_RUBY_PROJECT for Ruby integration checks")
class RuntimeCaptureTest(unittest.TestCase):
    def execute(self, source, runtime=None):
        return execute({"runtime": runtime or {}}, source, Path(PROJECT), ROOT)

    def test_constant_stderr_writes_are_captured(self):
        result = self.execute('STDERR.puts "notice"')
        self.assertEqual(base64.b64decode(result["stderr"]), b"notice\n")

    def test_constant_stdout_binary_writes_are_captured(self):
        result = self.execute('STDOUT.write("\\xff".b)')
        self.assertEqual(base64.b64decode(result["stdout"]), b"\xff")

    def test_exit_status_is_observable(self):
        self.assertEqual(self.execute('exit(7)')["exception"]["status"], 7)

    def test_file_state_is_observable_and_runs_are_isolated(self):
        runtime = {"files": {"sample": "old"}, "watch": ["sample"]}
        a = self.execute('File.write("sample", "new")', runtime)
        b = self.execute('nil', runtime)
        self.assertNotEqual(a["files"], b["files"])
        self.assertEqual(base64.b64decode(b["files"]["sample"]), b"old")

    def test_runtime_setup_rejects_parent_path_writes(self):
        result = self.execute('nil', {"files": {"../outside": "bad"}})
        self.assertIn("runner_error", result)

    def test_file_execution_honors_top_level_return_and_ensure(self):
        result = self.execute('begin\n puts "before"\n return 42\nensure\n puts "cleanup"\nend\nputs "unreachable"\n',
                              {"execution": "file"})
        self.assertNotIn("exception", result)
        self.assertEqual(base64.b64decode(result["stdout"]), b"before\ncleanup\n")

    def test_file_execution_preserves_argument_effects_and_watched_files(self):
        result = self.execute('return File.write("seen", "called")\n',
                              {"execution": "file", "watch": ["seen"]})
        self.assertNotIn("exception", result)
        self.assertEqual(base64.b64decode(result["files"]["seen"]), b"called")

    def test_unknown_execution_mode_is_a_runner_failure(self):
        result = self.execute('puts "unexpected"', {"execution": "other"})
        self.assertIn("runner_error", result)

    def test_literal_generator_covers_mode_quote_regressions(self):
        import tempfile
        from types import SimpleNamespace
        from parity_audit.literals import generate
        seeds = [{"cop": "Style/FileWrite", "source": "File.open('sample', 'w') { |f| f.write('new') }\n",
                  "path": "probe.rb", "config": {}}]
        args = SimpleNamespace(ruby_project=Path(PROJECT), timeout=30)
        with tempfile.TemporaryDirectory() as tmp:
            rows = generate(seeds, args, ROOT, Path(tmp))
        sources = [row["source"] for row in rows]
        self.assertTrue(any('"w\'"' in source for source in sources))
        self.assertTrue(any('%q|w|' in source for source in sources))
        self.assertTrue(any('\\x77' in source for source in sources))
