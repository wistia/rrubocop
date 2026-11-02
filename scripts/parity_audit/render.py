from collections import Counter
import json


def markdown(cases, names, results, summary):
    lines = ["# Custom parity audit", "", "```json", json.dumps(summary, indent=2), "```", "",
             "This is bounded differential testing, not proof of complete correctness.", "",
             "Diagnostics (including ranges/correctability) are compared before correction. "
             "Safe/all modes compare final source and Ruby compilation; only explicit runtime scenarios execute.", ""]
    lines.extend(coverage(cases, names))
    lines.extend(runtime_activity(summary.get("runtime_activity")))
    lines.extend(findings(results))
    return "\n".join(lines) + "\n"


def coverage(cases, names):
    counts = Counter(case["cop"] for case in cases)
    runtime = Counter(case["cop"] for case in cases if "runtime" in case)
    lines = ["## Selected coverage", "", "| Cop | Examples | Runtime scenarios selected |", "| --- | ---: | ---: |"]
    lines.extend(f"| {name} | {counts[name]} | {runtime[name]} |" for name in names)
    return lines


def runtime_activity(activity):
    if activity is None:
        return []
    lines = ["", "## Runtime correction activity", "",
             f"Artifact status: `{activity['status']}`. Scenario presence alone does not establish that a correction ran.", "",
             "Positive counts require a source change, valid Ruby compilation, normal native execution, "
             "and no native-only runtime regression. Shared reference behavior changes are reported separately. "
             "Unchanged scenarios include negative controls, report-only safeguards and corrections not applied in that mode.", "",
             "| Cop | Scenarios | Positive safe | Positive all | Unchanged safe | Unchanged all | Blocked mode runs |",
             "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |"]
    lines.extend(runtime_row(cop, row) for cop, row in activity["by_cop"].items())
    return lines


def runtime_row(cop, row):
    safe, all_modes = row["safe"], row["all"]
    values = (cop, row["scenarios"], safe["positive_rewrites"], all_modes["positive_rewrites"],
              unchanged_count(safe), unchanged_count(all_modes), safe["blocked"] + all_modes["blocked"])
    return "| " + " | ".join(str(value) for value in values) + " |"


def unchanged_count(mode):
    return sum(mode[key] for key in ("unchanged_negative", "unchanged_report_only", "unchanged_not_applied"))


def findings(results):
    groups = Counter((r["status"], r["cop"], r["kind"]) for r in results)
    lines = ["", "## Findings", "", "| Status | Cop | Kind | Count |", "| --- | --- | --- | ---: |"]
    lines.extend(f"| {status} | {cop} | {kind} | {count} |" for (status, cop, kind), count in sorted(groups.items()))
    lines.extend(["", "Sources/configurations: `cases.json`. Reproductions and exact differences: `results.json`. "
                  "Raw output: `native.jsonl`, `reference.jsonl`, `runtime.json`. "
                  "Build and bundle identity: `run-meta.json`, `reference-meta.json`.", ""])
    return lines
