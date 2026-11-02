import json

from .compare import load
from .runtime_observations import measure_case, runtime_records
from .runtime_activity_counts import MODES, by_mode, cop_counts, positive_cops


def runtime_activity(cases, names, args, output):
    selected = [case for case in cases if "runtime" in case]
    if not selected:
        return measure_runtime([], names, {}, {}, [], "not_requested")
    if args.skip_runtime:
        return measure_runtime(selected, names, {}, {}, [], "skipped")
    missing = missing_artifacts(output)
    if missing:
        result = measure_runtime(selected, names, {}, {}, [], "unavailable")
        result["missing_artifacts"] = missing
        return result
    return measure_artifacts(cases, selected, names, output)


def measure_runtime(cases, names, native, reference, records, status="complete"):
    indexed = runtime_records(cases, records)
    rows = [measure_case(case, mode, native, reference, indexed, status)
            for case in cases for mode in MODES]
    if status == "complete" and len(indexed) != len(cases) * len(MODES):
        status = "partial"
    return measured_summary(cases, names, rows, indexed, status)


def measured_summary(cases, names, rows, indexed, status):
    positive = [row for row in rows if row["activity"] == "positive_rewrites"]
    cops = sorted({case["cop"] for case in cases})
    supported = positive_cops(positive)
    return {
        "status": status,
        "selected_scenarios": len(cases),
        "recorded_mode_runs": len(indexed),
        **positive_summary(positive, supported, cops, names),
        "modes": by_mode(rows),
        "by_cop": {cop: cop_counts(cop, cases, rows) for cop in cops},
    }


def missing_artifacts(output):
    required = ("native.jsonl", "reference.jsonl", "runtime.json")
    return [name for name in required if not (output / name).exists()]


def measure_artifacts(cases, selected, names, output):
    native, reference = load(cases, output)
    return measure_runtime(selected, names, native, reference, json.loads((output / "runtime.json").read_text()))


def positive_summary(positive, supported, cops, names):
    return {"positive_rewrite_scenarios": len({row["id"] for row in positive}),
            "positive_rewrite_cops": supported,
            "scenario_only_cops": sorted(set(cops) - set(supported)),
            "cops_without_positive_runtime_rewrites": sorted(set(names) - set(supported))}
