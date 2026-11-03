#!/usr/bin/env python3
"""Compare pinned plugin availability through actual Ruby/native CLI entrypoints."""
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
observations = []
with tempfile.TemporaryDirectory(prefix='plugin-registration-', dir='/dev/shm') as temporary:
    root = Path(temporary)
    (root / 'Gemfile.lock').write_text('GEM\n  specs:\n    rubocop (1.65.1)\n    rubocop-graphql (1.5.4)\n')
    for label, relative, disabled, expected in [
        ('registered-without-default', 'app/graphql/check.rb', False, 1),
        ('user-disabled', 'app/graphql/check.rb', True, 0),
        ('outside-plugin-scope', 'app/models/check.rb', False, 0),
    ]:
        source = root / relative
        source.parent.mkdir(parents=True, exist_ok=True)
        source.write_text('argument :status, String, prepare: Transformer\n')
        config = root / '.rubocop.yml'
        config.write_text('require: rubocop-graphql\nAllCops:\n  NewCops: disable\n' +
                          ('GraphQL/PrepareMethod:\n  Enabled: false\n' if disabled else ''))
        row = {'control': label, 'expected_offenses': expected}
        for engine, command in [('ruby', ['bin/rubocop-ruby']), ('native', [str(args.native.resolve())])]:
            selection = [] if disabled else ['--only', 'GraphQL/PrepareMethod']
            result = subprocess.run(command + ['--config', str(config)] + selection +
                                    ['--format', 'json', '--cache', 'false', str(source)],
                                    cwd=args.ruby_project, capture_output=True, text=True,
                                    env=os.environ | {'ASDF_RUBY_VERSION': '3.4.9',
                                                      'RRUBOCOP_CACHE_DIR': str(root / 'native-cache'),
                                                      'RUBOCOP_CACHE_ROOT': str(root / 'ruby-cache')})
            (args.output / f'{label}-{engine}.stdout').write_text(result.stdout)
            (args.output / f'{label}-{engine}.stderr').write_text(result.stderr)
            data = json.loads(result.stdout)
            offenses = (data['offenses'] if engine == 'native' else
                        [offense for file in data['files'] for offense in file['offenses']])
            row[engine] = {'exit': result.returncode, 'offenses': sum(offense['cop_name'] == 'GraphQL/PrepareMethod' for offense in offenses)}
        row['passed'] = all(row[engine]['offenses'] == expected and
                            (disabled or row[engine]['exit'] == int(expected > 0))
                            for engine in ('ruby', 'native'))
        observations.append(row)
summary = {'controls': observations, 'failures': sum(not row['passed'] for row in observations)}
(args.output / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
print(json.dumps(summary, indent=2))
raise SystemExit(bool(summary['failures']))
