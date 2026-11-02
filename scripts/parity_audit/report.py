from collections import Counter
import json

from .render import markdown
from .runtime_coverage import runtime_activity


def coverage_counts(cases):
    return Counter(case["cop"] for case in cases)


def summary(cases, names, results, args):
    result = selection_coverage(cases, names)
    result.update(status_counts(results))
    result.update(comparison_coverage(cases, names, results))
    result.update(runtime_coverage(cases, names, args))
    return result


def selection_coverage(cases, names):
    coverage = coverage_counts(cases)
    return {"cases": len(cases), "covered_cops": len(coverage), "available_cops": len(names),
              "missing_cops": sorted(set(names) - coverage.keys())}


def runtime_coverage(cases, names, args):
    runtime = [case for case in cases if "runtime" in case and not args.skip_runtime]
    cops = sorted({case["cop"] for case in runtime})
    return {"runtime_cases": len(runtime), "runtime_cops": cops,
            "missing_runtime_cops": sorted(set(names) - set(cops)),
            "selection_applied": bool(args.cop or args.limit or args.case_id or args.runtime_only)}


def write_report(cases, names, results, output, args):
    result = summary(cases, names, results, args)
    result["runtime_activity"] = runtime_activity(cases, names, args, output)
    write_json(output / "results.json", results)
    write_json(output / "summary.json", result)
    (output / "report.md").write_text(markdown(cases, names, results, result))
    return result


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n")


def status_counts(results):
    statuses = Counter(result["status"] for result in results)
    return {"new_failures": statuses["failure"], "documented_exceptions": statuses["exception"],
            "upstream_hazards": statuses["upstream_hazard"], "invalid_mutations": statuses["invalid_mutation"],
            "invalid_originals": statuses["invalid_original"]}


def comparison_coverage(cases, names, results):
    excluded = {r["id"] for r in results if r["kind"] in ("reference_error", "original_invalid_ruby")}
    compared = [c for c in cases if c["id"] not in excluded]
    cops = {c["cop"] for c in compared}
    return {"compared_cases": len(compared), "compared_cops": len(cops),
            "uncompared_cops": sorted(set(names) - cops)}
