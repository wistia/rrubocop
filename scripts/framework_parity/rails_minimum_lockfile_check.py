#!/usr/bin/env python3
"""Reuse the final-Rails real-file control setup for independent target/gem minimum gates."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--ruby-project', type=Path, required=True)
parser.add_argument('--native', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=True)
rows = []
with tempfile.TemporaryDirectory(prefix='rrubocop-framework-minimum-', dir='/dev/shm') as directory:
    root = Path(directory)
    env = {**os.environ, 'ASDF_RUBY_VERSION': '3.4.9',
           'BUNDLE_GEMFILE': str(args.ruby_project / 'Gemfile')}
    controls = [('Rails/ContentTag', (5, 1), 'tag(:br)', 'app/helpers/probe.rb', 5.0),
                ('Rails/IndexWith', (6, 0), 'x.map { |el| [el, el.to_sym] }.to_h', 'app/models/probe.rb', 5.1),
                ('Rails/Pick', (6, 0), 'User.pluck(:name).first', 'app/models/probe.rb', 5.1)]
    for cop, minimum, text, path, low_target in controls:
        source = root / path
        source.parent.mkdir(parents=True, exist_ok=True)
        source.write_text(text + '\n')
        config = root / '.rubocop.yml'
        for target in (low_target, 8.1):
            config.write_text(f'require: [rubocop-rails]\nAllCops:\n  TargetRubyVersion: 3.4\n'
                              f'  TargetRailsVersion: {target}\n  DisabledByDefault: true\n'
                              f'{cop}:\n  Enabled: true\n')
            for version in (None, '4.2.11.3', '5.0.0', '5.1.0', '6.0.0', '8.1.3.1'):
                lock = root / 'Gemfile.lock'
                if version is None:
                    lock.unlink(missing_ok=True)
                else:
                    lock.write_text(f'GEM\n  remote: https://rubygems.org/\n  specs:\n'
                                    f'    railties ({version})\n\nPLATFORMS\n  ruby\n\nDEPENDENCIES\n  railties\n')
                observations = []
                for command in ([str(args.ruby_project / 'bin/rubocop-ruby')], [str(args.native)]):
                    result = subprocess.run(command + ['--config', str(config), '--only', cop,
                                                      '--format', 'json', '--cache', 'false', str(source)],
                                            cwd=root, env=env, capture_output=True, text=True, timeout=30)
                    if result.returncode not in (0, 1):
                        raise RuntimeError(result.stderr)
                    value = json.loads(result.stdout)
                    offenses = value.get('offenses')
                    if offenses is None:
                        offenses = [offense for file in value.get('files', []) for offense in file['offenses']]
                    observations.append(sum(offense['cop_name'] == cop for offense in offenses))
                gem = tuple(int(part) for part in version.split('.')[:2]) if version else None
                target_version = tuple(int(part) for part in str(target).split('.'))
                expected = int(target_version >= minimum and gem is not None and gem >= minimum)
                rows.append(dict(cop=cop, target=target, railties=version, ruby=observations[0],
                                 native=observations[1], expected=expected,
                                 matches=observations == [expected, expected]))
(args.output / 'results.json').write_text(json.dumps(rows, indent=2) + '\n')
summary = dict(controls=len(rows), differences=sum(not row['matches'] for row in rows))
(args.output / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
print(json.dumps(summary))
raise SystemExit(bool(summary['differences']))
