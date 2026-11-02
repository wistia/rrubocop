#!/usr/bin/env python3
"""Refresh owned-rule evidence from strict differential artifacts, preserving failures."""
import argparse
from collections import Counter
import json
from pathlib import Path
import re


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('results', nargs='+', type=Path,
                        help='Strict result directories, oldest first; newer observations replace matching IDs')
    parser.add_argument('--report', type=Path, default=Path(__file__).resolve().parents[2] /
                        'docs/reviews/performance-seven-parity.json')
    args = parser.parse_args()
    report = json.loads(args.report.read_text())
    cases, failures, runtime = {}, {}, {}
    for directory in args.results:
        rows = json.loads((directory / 'cases.json').read_text())
        grouped = {}
        for failure in json.loads((directory / 'failures.json').read_text()):
            grouped.setdefault(failure['id'], []).append(failure)
        for row in rows:
            cases[row['id']] = row
            failures[row['id']] = grouped.get(row['id'], [])
        for row in json.loads((directory / 'runtime.json').read_text()):
            runtime[row['id']] = row
    totals = Counter()
    for rule in report['rules']:
        cop = rule['cop']
        rows = [row for row in cases.values() if row['cop'] == cop]
        # cases.json contains originals AND generated states. Count them separately.
        originals = [row for row in rows if not row['id'].endswith(':second-pass')]
        states = [row for row in rows if row['id'].endswith(':second-pass')]
        findings = [finding for row in rows for finding in failures[row['id']]]
        kinds = Counter(finding['kind'] for finding in findings)
        def case_failures(row):
            return [finding for suffix in ('', ':safe:second-pass', ':all:second-pass')
                    for finding in failures.get(row['id'] + suffix, [])]
        stem = re.sub(r'(?<!^)(?=[A-Z])', '_', cop.split('/')[1]).lower()
        rule.update(status='implemented_source_reviewed_with_safeguards',
                    original_source=f'rubocop-performance-1.21.1/lib/rubocop/cop/performance/{stem}.rb',
                    original_spec=f'tests/fixtures/performance-seven-upstream/{stem}_spec.rb',
                    original_cases=len(originals), second_pass_cases=len(states),
                    exact_original_cases=sum(not case_failures(row) for row in originals),
                    diagnostic_differences=kinds['diagnostics'],
                    invalid_native_corrections=kinds['invalid_native_correction'],
                    idempotence_differences=kinds['non_idempotent'],
                    original_invalid_states=kinds['original_invalid'],
                    strict_driver_exit=int(bool(findings)), strict_driver_failures=dict(kinds),
                    trusted_runtime_programs=sum(row.get('trusted_runtime', False) for row in originals))
        rule['safeguards'] = [dict(id=row['id'], source=row['source'], config=row.get('config', {}),
                                  failure_kinds=sorted({finding['kind'] for finding in case_failures(row)}))
                              for row in originals if case_failures(row)]
        trusted = [runtime[row['id']] for row in originals if row.get('trusted_runtime')]
        rule['all_native_runtime_states_preserve_original'] = all(
            mode['native_preserves_original'] for row in trusted for mode in row['modes'].values())
        totals.update(original_cases=len(originals), second_pass_cases=len(states),
                      trusted_runtime_programs=len(trusted), diagnostic_differences=kinds['diagnostics'],
                      invalid_native_corrections=kinds['invalid_native_correction'],
                      idempotence_differences=kinds['non_idempotent'])
    report['totals'] = dict(totals)
    report['totals']['implemented_rules'] = len(report['rules'])
    report['report_counting'] = 'Original case IDs exclude generated :second-pass IDs; findings retain all states. Later observations replace earlier matching IDs, including resolved failures.'
    args.report.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report['totals'], indent=2))


if __name__ == '__main__':
    main()
