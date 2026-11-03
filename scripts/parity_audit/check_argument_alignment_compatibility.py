"""Check sibling HashAlignment config injection through the real CLI."""
import argparse
import itertools
import json
import os
import pathlib
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
for name in ('ruby', 'rust', 'gemfile', 'output'):
    parser.add_argument('--' + name, type=pathlib.Path, required=True)
settings = parser.parse_args()
cop = 'Layout/ArgumentAlignment'
source = 'foo(one,\n two)\n'
env = dict(os.environ, BUNDLE_GEMFILE=str(settings.gemfile.resolve()))
rows = []
siblings = [
    ('key', True, 'key', 'key', False),
    ('colon-separator', True, 'separator', 'key', True),
    ('rocket-separator', True, 'key', 'separator', True),
    ('colon-sequence', True, ['key', 'separator'], 'key', True),
    ('rocket-sequence', True, 'key', ['table', 'separator'], True),
    ('disabled', False, 'separator', 'separator', False),
]
with tempfile.TemporaryDirectory() as directory:
    root = pathlib.Path(directory)
    path, config = root / 'input.rb', root / 'config.yml'
    for (label, enabled, colon, rocket, separator), style, mode in itertools.product(siblings, ['with_first_argument', 'with_fixed_indentation'], ['-a', '-A']):
        skip = style == 'with_first_argument' and separator
        expected = source if skip else source.replace('\n two', '\n' + ' ' * (4 if style == 'with_first_argument' else 2) + 'two')
        config.write_text(json.dumps({
            'AllCops': {'DisabledByDefault': True, 'NewCops': 'disable', 'TargetRubyVersion': 3.4},
            cop: {'Enabled': True, 'EnforcedStyle': style, 'IndentationWidth': 2},
            'Layout/HashAlignment': {'Enabled': enabled, 'EnforcedColonStyle': colon, 'EnforcedHashRocketStyle': rocket},
        }))
        observations = []
        for engine, exe in [('ruby', settings.ruby.resolve()), ('rust', settings.rust.resolve())]:
            path.write_text(source)
            command = [str(exe), '--config', str(config), '--only', cop, '--cache', 'false', '--format', 'json', mode]
            if engine == 'rust':
                command.extend(['--rubocop-version', '1.65.1'])
            process = subprocess.run(command + [str(path)], cwd=settings.gemfile.resolve().parent, env=env, capture_output=True, text=True)
            assert process.returncode == 0, (label, style, engine, process.stdout, process.stderr)
            report = json.loads(process.stdout)
            offenses = report['files'][0]['offenses'] if 'files' in report else report['offenses']
            assert len(offenses) == (0 if skip else 1), (label, style, engine, report)
            assert all(offense['corrected'] for offense in offenses), (label, engine, offenses)
            assert path.read_text() == expected, (label, style, engine, path.read_text(), expected)
            observations.append({'engine': engine, 'exit': process.returncode, 'offenses': len(offenses), 'bytes': path.read_text()})
        rows.append({'case': label, 'style': style, 'mode': mode, 'observations': observations})
settings.output.write_text(json.dumps(rows, indent=2) + '\n')
print('compatibility controls', len(rows), 'all match')
