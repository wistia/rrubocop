"""Recheck all cops on the physical files from the recorded 091dc33 discrepancy."""
import argparse
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--ruby-project', type=Path, required=True)
parser.add_argument('--native', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True)
parser.add_argument('--manifest', type=Path, help='JSON target list for a full same-file comparison')
parser.add_argument('--baseline', type=Path, default=Path(__file__).resolve().parents[2] /
                    'docs/evidence/091dc33-real-corpus-comparison.json')
args = parser.parse_args()
root = args.ruby_project.resolve()
args.output.mkdir(parents=True, exist_ok=True)
baseline = json.loads(args.baseline.read_text())
invocation = baseline['invocation']
source_head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip()
assert source_head == invocation['wista_head'], (source_head, invocation['wista_head'])
paths = json.loads(args.manifest.read_text()) if args.manifest else invocation['ruby_argv'][5:]
assert paths and len(paths) == len(set(paths))
assert args.manifest or len(paths) == 74
before = {path: hashlib.sha256((root / path).read_bytes()).hexdigest() for path in paths}
environment = {'ASDF_RUBY_VERSION': '3.4.9', 'BUNDLE_GEMFILE': str(root / 'Gemfile'),
               'RRUBOCOP_CACHE_DIR': str(args.output.resolve() / 'native-cache'),
               'RUBOCOP_CACHE_ROOT': str(args.output.resolve() / 'ruby-cache')}
severity = {'C': 'convention', 'W': 'warning', 'E': 'error', 'F': 'fatal', 'R': 'refactor'}
observations = {}

def normalize(engine, path, offense):
    absolute = Path(path) if Path(path).is_absolute() else root / path
    location = offense.get('location')
    return {'path': str(absolute.relative_to(root)), 'cop_name': offense['cop_name'],
            'line': location['start_line'] if location else offense['line'],
            'column': location['start_column'] - 1 if location else offense['column'],
            'severity': severity.get(offense['severity'], offense['severity']),
            'message': (offense['message'].removeprefix(offense['cop_name'] + ': ')
                        if engine == 'native' else offense['message'])}

for engine, executable in [('ruby', root / 'bin/rubocop-ruby'), ('native', args.native.resolve())]:
    command = [str(executable), '--cache', 'false', '--format', 'json', *paths]
    process = subprocess.run(command, cwd=root, env=os.environ | environment,
                             capture_output=True, text=True, timeout=900)
    (args.output / f'{engine}.stdout').write_text(process.stdout)
    (args.output / f'{engine}.stderr').write_text(process.stderr)
    report = json.loads(process.stdout)
    if engine == 'ruby':
        offenses = [normalize(engine, file['path'], offense) for file in report['files'] for offense in file['offenses']]
        inspected = report['summary']['inspected_file_count']
    else:
        offenses = [normalize(engine, offense['path'], offense) for offense in report['offenses']]
        inspected = report['metadata']['files_inspected']
    observations[engine] = {'argv': command, 'cwd': str(root), 'environment_overrides': environment,
                            'exit': process.returncode, 'files_inspected': inspected,
                            'offenses': sorted(offenses, key=lambda row: tuple(row.values())),
                            'stderr': process.stderr}
after = {path: hashlib.sha256((root / path).read_bytes()).hexdigest() for path in paths}
assert before == after, 'Source files changed during comparison'
def keys(engine):
    return Counter(json.dumps(offense, sort_keys=True) for offense in observations[engine]['offenses'])
reference, native = keys('ruby'), keys('native')
summary = {'source_head': source_head, 'files': len(paths), 'source_sha256': before,
           'diagnostic_fields': ['path', 'cop_name', 'line', 'column', 'severity', 'message'],
           'ruby_offenses': sum(reference.values()), 'native_offenses': sum(native.values()),
           'ruby_only': [json.loads(key) for key in (reference - native).elements()],
           'native_only': [json.loads(key) for key in (native - reference).elements()],
           'observations': observations}
summary['passed'] = reference == native and all(
    row['files_inspected'] == len(paths) and row['exit'] in (0, 1) and not row['stderr']
    for row in observations.values())
(args.output / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
print(json.dumps({key: summary[key] for key in ['files', 'ruby_offenses', 'native_offenses',
                                              'ruby_only', 'native_only', 'passed']}, indent=2))
raise SystemExit(not summary['passed'])
