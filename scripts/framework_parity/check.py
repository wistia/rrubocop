#!/usr/bin/env python3
"""Strict differential checks; every difference remains visible and fails the run."""
import argparse
import json
import os
from pathlib import Path
import subprocess
from cases import cases

parser = argparse.ArgumentParser()
parser.add_argument('--ruby-project', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
root = Path(__file__).resolve().parents[2]
args.output.mkdir(parents=True, exist_ok=True)

def native(rows):
    (args.output / 'cases.json').write_text(json.dumps(rows))
    env = {**os.environ, 'RRUBOCOP_AUDIT_INPUT': str(args.output / 'cases.json'), 'RRUBOCOP_AUDIT_OUTPUT': str(args.output / 'native.jsonl')}
    subprocess.run(['cargo', 'test', '--locked', '--lib', 'custom_parity_audit_probe', '--', '--ignored'], cwd=root, env=env, check=True)
    return {(r['id'], r['mode']): r['result'] for r in map(json.loads, (args.output / 'native.jsonl').read_text().splitlines())}

rows = cases()
initial = native(rows)
state_rows = []
for case in rows:
    for mode in ('safe', 'all'):
        state_rows.append({**case, 'id': case['id'] + ':' + mode + ':second-pass', 'source': initial[case['id'], mode].get('source', case['source'])})
all_rows = rows + state_rows
results = native(all_rows)
subprocess.run(['bundle', 'exec', 'ruby', str(root / 'scripts/parity_audit/reference.rb'), str(args.ruby_project / '.rubocop.yml'), str(args.output / 'cases.json'), str(args.output / 'reference.jsonl')], cwd=args.ruby_project, check=True)
reference = {r['id']: r['result'] for r in map(json.loads, (args.output / 'reference.jsonl').read_text().splitlines())}
failures = []
for case in all_rows:
    name = case['id']
    ruby = reference[name]
    rust = results[name, 'off']
    if 'error' in ruby or 'error' in rust:
        failures.append(dict(id=name, kind='error', ruby=ruby, rust=rust))
        continue
    rd = [(o['line'], o['column'], o['message'], o['length'], o['severity'], o['correctable']) for o in ruby['diagnostics']]
    nd = [(o['location']['line'], o['location']['column'], o['message'], o.get('highlight_length', 1), o['severity'].lower(), o['correctable']) for o in rust['diagnostics']]
    if rd != nd:
        failures.append(dict(id=name, source=case['source'], kind='diagnostics', ruby=rd, rust=nd))
    for mode in ('safe', 'all'):
        got = results[name, mode]
        expected = ruby[mode]
        if got.get('source') != expected.get('source') or 'error' in got or 'error' in expected:
            failures.append(dict(id=name, source=case['source'], kind=mode, ruby=expected, rust=got))
        if not ruby.get('native_compile', {}).get(mode, {}).get('valid', False):
            failures.append(dict(id=name, kind='invalid_native_correction', mode=mode))
for case in rows:
    for mode in ('safe', 'all'):
        name = case['id'] + ':' + mode + ':second-pass'
        if results[name, mode].get('source') != initial[case['id'], mode].get('source'):
            failures.append(dict(id=case['id'], kind='non_idempotent', mode=mode))
runtime_results = []
def observe(source):
    result = subprocess.run(['ruby', '-e', source], cwd=args.ruby_project, capture_output=True, text=True, timeout=5)
    return dict(exit_code=result.returncode, stdout=result.stdout, stderr=result.stderr)
for case in rows:
    if not case.get('trusted_runtime'):
        continue
    original = observe(case['source'])
    row = dict(id=case['id'], original=original, modes={})
    for mode in ('safe', 'all'):
        expected = reference[case['id']].get(mode)
        rust = observe(results[case['id'], mode]['source'])
        if not isinstance(expected, dict) or 'source' not in expected:
            unavailable = dict(unavailable=True, error=reference[case['id']].get('error', 'No corrected source returned'))
            row['modes'][mode] = dict(ruby=unavailable, rust=rust, equivalent=False,
                                     native_preserves_original=original['exit_code'] == rust['exit_code'] and original['stdout'] == rust['stdout'])
            failures.append(dict(id=case['id'], kind='runtime_reference_unavailable', mode=mode, ruby=unavailable, rust=rust))
            continue
        ruby = observe(expected['source'])
        equal = ruby['exit_code'] == rust['exit_code'] and ruby['stdout'] == rust['stdout']
        row['modes'][mode] = dict(ruby=ruby, rust=rust, equivalent=equal, native_preserves_original=original['exit_code'] == rust['exit_code'] and original['stdout'] == rust['stdout'])
        if not equal:
            failures.append(dict(id=case['id'],kind='runtime',mode=mode,ruby=ruby,rust=rust))
    runtime_results.append(row)
(args.output / 'runtime.json').write_text(json.dumps(runtime_results, indent=2))
(args.output / 'failures.json').write_text(json.dumps(failures, indent=2))
summary = dict(original_cases=len(rows), second_pass_cases=len(state_rows), trusted_runtime_scenarios=len(runtime_results), failures=len(failures), failing_cops=sorted(set(f['id'].split(':')[0] for f in failures)))
(args.output / 'summary.json').write_text(json.dumps(summary, indent=2))
print(json.dumps(summary, indent=2))
raise SystemExit(bool(failures))
