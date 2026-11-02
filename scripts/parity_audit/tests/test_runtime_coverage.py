import json
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from parity_audit.report import write_report
from parity_audit.runtime_coverage import measure_runtime, runtime_activity


def case(name="Style/Example"):
    return {"id": name, "cop": name, "source": "original", "runtime": {},
            "corrected": "claimed rewrite", "origin": "reference"}


def artifacts(cases):
    native, reference, records = {}, {}, []
    for row in cases:
        reference[row["id"]] = {"original_compile": {"valid": True},
                                "native_compile": {mode: {"valid": True} for mode in ("safe", "all")}}
        for mode in ("off", "safe", "all"):
            native[(row["id"], mode)] = {"source": row["source"], "diagnostics": []}
        for mode in ("safe", "all"):
            records.append({"id": row["id"], "mode": mode,
                            "original": {"stdout": "same"}, "native": {"stdout": "same"}, "ruby": {"stdout": "same"}})
    return native, reference, records


def measured(cases, native, reference, records):
    return measure_runtime(cases, [row["cop"] for row in cases], native, reference, records)


class RuntimeActivityTest(unittest.TestCase):
    def test_seven_present_but_never_rewritten_rules_have_no_positive_coverage(self):
        names = ["Lint/BooleanSymbol", "Lint/ErbNewArguments", "Style/ClassEqualityComparison",
                 "Style/HashExcept", "Style/RandomWithOffset", "Style/SlicingWithRange", "Style/UnpackFirst"]
        cases = [case(name) for name in names]
        result = measured(cases, *artifacts(cases))
        self.assertEqual(result["selected_scenarios"], 7)
        self.assertEqual(result["positive_rewrite_scenarios"], 0)
        self.assertEqual(result["positive_rewrite_cops"], [])
        self.assertEqual(result["scenario_only_cops"], sorted(names))
        self.assertEqual(result["modes"]["all"]["unchanged_negative"], 7)

    def test_safe_and_all_counts_follow_actual_source_not_fixture_claims(self):
        row = case()
        native, reference, records = artifacts([row])
        native[(row["id"], "off")]["diagnostics"] = [{"correctable": True}]
        native[(row["id"], "all")]["source"] = "actual rewrite"
        result = measured([row], native, reference, records)
        self.assertEqual(result["modes"]["safe"]["positive_rewrites"], 0)
        self.assertEqual(result["modes"]["safe"]["unchanged_not_applied"], 1)
        self.assertEqual(result["modes"]["all"]["positive_rewrites"], 1)
        self.assertEqual(result["positive_rewrite_scenarios"], 1)

    def test_unchanged_negative_and_report_only_cases_are_separate(self):
        cases = [case("Style/Negative"), case("Style/Guard")]
        native, reference, records = artifacts(cases)
        native[(cases[1]["id"], "off")]["diagnostics"] = [{"correctable": False}]
        result = measured(cases, native, reference, records)
        for mode in ("safe", "all"):
            self.assertEqual(result["modes"][mode]["unchanged_negative"], 1)
            self.assertEqual(result["modes"][mode]["unchanged_report_only"], 1)

    def test_runner_errors_invalid_code_and_program_exceptions_are_not_positive(self):
        mutations = {
            "native_error": lambda n, r, rows, key: n[key].update(error="panic"),
            "off_error": lambda n, r, rows, key: n[(key[0], "off")].update(error="panic"),
            "original_invalid": lambda n, r, rows, key: r[key[0]]["original_compile"].update(valid=False),
            "native_invalid": lambda n, r, rows, key: r[key[0]]["native_compile"]["all"].update(valid=False),
            "missing_compile": lambda n, r, rows, key: r[key[0]].pop("native_compile"),
            "runner_error": lambda n, r, rows, key: rows[1]["native"].update(runner_error="timeout"),
            "reference_runner_error": lambda n, r, rows, key: rows[1]["ruby"].update(runner_error="timeout"),
            "native_exception": lambda n, r, rows, key: rows[1]["native"].update(exception={"class": "NameError"}),
            "original_exception": lambda n, r, rows, key: rows[1]["original"].update(exception={"class": "NameError"}),
            "missing_source": lambda n, r, rows, key: n[key].pop("source"),
        }
        for label, mutate in mutations.items():
            with self.subTest(label=label):
                row = case()
                native, reference, records = artifacts([row])
                key = (row["id"], "all")
                native[key]["source"] = "changed"
                mutate(native, reference, records, key)
                result = measured([row], native, reference, records)
                self.assertEqual(result["modes"]["all"]["positive_rewrites"], 0)
                self.assertEqual(result["modes"]["all"]["blocked"], 1)

    def test_shared_behavior_change_is_positive_but_native_only_failure_is_not(self):
        row = case()
        native, reference, records = artifacts([row])
        native[(row["id"], "all")]["source"] = "changed"
        records[1]["native"] = records[1]["ruby"] = {"stdout": "new"}
        result = measured([row], native, reference, records)
        self.assertEqual(result["modes"]["all"]["positive_rewrites"], 1)
        self.assertEqual(result["modes"]["all"]["behavior_outcomes"], {"shared_upstream_change": 1})
        records[1]["ruby"] = {"stdout": "other"}
        result = measured([row], native, reference, records)
        self.assertEqual(result["modes"]["all"]["positive_rewrites"], 0)
        self.assertEqual(result["modes"]["all"]["blocked_reasons"], {"native_behavior_failure": 1})

    def test_valid_native_rewrite_can_preserve_behavior_when_reference_raises(self):
        row = case()
        native, reference, records = artifacts([row])
        native[(row["id"], "all")]["source"] = "safe rewrite"
        records[1]["ruby"] = {"exception": {"class": "SyntaxError"}}
        result = measured([row], native, reference, records)
        self.assertEqual(result["modes"]["all"]["positive_rewrites"], 1)
        self.assertEqual(result["modes"]["all"]["behavior_outcomes"], {"preserved_despite_reference_change": 1})

    def test_shared_exception_is_not_a_completed_positive_rewrite(self):
        row = case()
        native, reference, records = artifacts([row])
        native[(row["id"], "all")]["source"] = "changed"
        records[1]["native"] = records[1]["ruby"] = {"exception": {"class": "NoMethodError"}}
        result = measured([row], native, reference, records)
        self.assertEqual(result["modes"]["all"]["positive_rewrites"], 0)
        self.assertEqual(result["modes"]["all"]["blocked_reasons"], {"native_exception": 1})
        self.assertEqual(result["modes"]["all"]["behavior_outcomes"], {"shared_upstream_change": 1})

    def test_missing_mode_is_partial_not_positive(self):
        row = case()
        native, reference, records = artifacts([row])
        native[(row["id"], "all")]["source"] = "changed"
        result = measured([row], native, reference, records[:1])
        self.assertEqual(result["status"], "partial")
        self.assertEqual(result["modes"]["all"]["positive_rewrites"], 0)
        self.assertEqual(result["modes"]["all"]["blocked_reasons"], {"missing_execution": 1})

    def test_duplicate_unknown_or_incomplete_runtime_observations_fail(self):
        row = case()
        native, reference, records = artifacts([row])
        bad = [records + records[:1], [{**records[0], "id": "other"}],
               [{"id": row["id"], "mode": "all", "native": {}}]]
        for observations in bad:
            with self.subTest(observations=observations), self.assertRaises(ValueError):
                measured([row], native, reference, observations)


class RuntimeArtifactReportTest(unittest.TestCase):
    def test_missing_artifacts_or_skipped_runtime_never_claim_validated_rewrites(self):
        row = case()
        for skipped, status in [(False, "unavailable"), (True, "skipped")]:
            with self.subTest(skipped=skipped), tempfile.TemporaryDirectory() as tmp:
                result = runtime_activity([row], [row["cop"]], SimpleNamespace(skip_runtime=skipped), Path(tmp))
                self.assertEqual(result["status"], status)
                self.assertEqual(result["positive_rewrite_scenarios"], 0)

    def test_report_reads_real_artifacts_and_retains_presence_fields(self):
        row = case()
        native, reference, records = artifacts([row])
        native[(row["id"], "all")]["source"] = "actual rewrite"
        args = SimpleNamespace(skip_runtime=False, cop=None, limit=None, case_id=None, runtime_only=False)
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp)
            native_rows = [{"id": key[0], "mode": key[1], "result": value} for key, value in native.items()]
            (output / "native.jsonl").write_text("\n".join(json.dumps(value) for value in native_rows))
            (output / "reference.jsonl").write_text(json.dumps({"id": row["id"], "result": reference[row["id"]]}))
            (output / "runtime.json").write_text(json.dumps(records))
            result = write_report([row], [row["cop"]], [], output, args)
            self.assertEqual(result["runtime_cases"], 1)
            self.assertEqual(result["runtime_cops"], [row["cop"]])
            self.assertEqual(result["missing_runtime_cops"], [])
            self.assertEqual(result["runtime_activity"]["modes"]["all"]["positive_rewrites"], 1)
            self.assertIn("Runtime correction activity", (output / "report.md").read_text())
