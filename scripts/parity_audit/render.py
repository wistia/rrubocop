from collections import Counter
import json


def markdown(cases, names, results, summary):
    lines = ["# Custom parity audit", "", "```json", json.dumps(summary, indent=2), "```", "",
             "This is bounded differential testing, not proof of complete correctness.", "",
             "Diagnostics (including ranges/correctability) are compared before correction. "
             "Safe/all modes compare final source and Ruby compilation; only explicit runtime scenarios execute.", ""]
    lines.extend(coverage(cases, names))
    lines.extend(findings(results))
    return "\n".join(lines) + "\n"


def coverage(cases, names):
    counts = Counter(case["cop"] for case in cases)
    runtime = Counter(case["cop"] for case in cases if "runtime" in case)
    lines = ["## Selected coverage", "", "| Cop | Examples | Runtime scenarios selected |", "| --- | ---: | ---: |"]
    lines.extend(f"| {name} | {counts[name]} | {runtime[name]} |" for name in names)
    return lines


def findings(results):
    groups = Counter((r["status"], r["cop"], r["kind"]) for r in results)
    lines = ["", "## Findings", "", "| Status | Cop | Kind | Count |", "| --- | --- | --- | ---: |"]
    lines.extend(f"| {status} | {cop} | {kind} | {count} |" for (status, cop, kind), count in sorted(groups.items()))
    lines.extend(["", "Sources/configurations: `cases.json`. Reproductions and exact differences: `results.json`. "
                  "Raw output: `native.jsonl`, `reference.jsonl`, `runtime.json`. "
                  "Build and bundle identity: `run-meta.json`, `reference-meta.json`.", ""])
    return lines
