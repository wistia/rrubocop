#!/usr/bin/env python3
"""Batch differential and behavior audit; see docs/custom-parity-audit.md."""
import json
from pathlib import Path
import subprocess
import sys

from parity_audit.cli import arguments
from parity_audit.cases import collect, select
from parity_audit.literals import expand
from parity_audit.compare import compare
from parity_audit.runtime import check_runtime
from parity_audit.report import write_report
from parity_audit.runner import engines, clear_results


def main():
    args = arguments()
    root = Path(__file__).resolve().parents[1]
    cases, names, output = prepare(args, root)
    engines(args, root, output)
    return finish(args, root, output, cases, names)


def prepare(args, root):
    output = args.output.resolve()
    prepare_output(output)
    cases, names = collect(root, args)
    cases = select(expand(cases, args, root, output), args)
    write_cases(output, cases)
    return cases, names, output


def prepare_output(output):
    if not reusable(output):
        raise ValueError("Output must be empty or a previous audit directory")
    output.mkdir(parents=True, exist_ok=True)
    (output / ".audit-owned").write_text("rrubocop parity audit\n")
    clear_results(output)


def reusable(output):
    return (not output.exists() or not any(output.iterdir()) or
            (output / ".audit-owned").exists() or (output / "run-meta.json").exists())


def write_cases(output, cases):
    (output / "cases.json").write_text(json.dumps(cases, indent=2) + "\n")
    print(f"Auditing {len(cases)} examples across {len({c['cop'] for c in cases})} cops", flush=True)


def finish(args, root, output, cases, names):
    results = compare(cases, output)
    if not args.skip_runtime:
        results.extend(check_runtime(cases, output, args.ruby_project.resolve(), root))
    summary = write_report(cases, names, results, output, args)
    show(summary, output)
    return exit_status(summary, args)


def show(summary, output):
    print(json.dumps(console_summary(summary), indent=2))
    print(f"Report: {output / 'report.md'}")


def console_summary(summary):
    keys = ("cases", "compared_cases", "covered_cops", "compared_cops", "available_cops", "new_failures",
            "documented_exceptions", "upstream_hazards", "invalid_mutations", "invalid_originals", "runtime_cases", "selection_applied")
    result = {key: summary[key] for key in keys}
    result["runtime_cop_count"] = len(summary["runtime_cops"])
    return result


def exit_status(summary, args):
    if summary["compared_cases"] == 0:
        return 2
    failed = summary["new_failures"] > 0
    return int(failed or args.fail_on_upstream_hazards and summary["upstream_hazards"] > 0)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (subprocess.SubprocessError, OSError, ValueError, KeyError, TypeError) as error:
        print(f"Audit could not complete: {error}", file=sys.stderr)
        sys.exit(2)
