"""Compare config validation and correction bytes without executing specimen programs."""
import argparse
import concurrent.futures
import json
import os
import pathlib
import re
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
for option in ('ruby', 'rust', 'gemfile', 'fixture', 'output'):
    parser.add_argument('--' + option, type=pathlib.Path, required=True)
parser.add_argument('--jobs', type=int, default=4)
settings = parser.parse_args()
environment = dict(os.environ, BUNDLE_GEMFILE=str(settings.gemfile.resolve()))
root = settings.gemfile.resolve().parent
cases = json.loads(settings.fixture.read_text())


def inspect(pair):
    case, mode = pair
    observations = {}
    with tempfile.TemporaryDirectory() as directory:
        base = pathlib.Path(directory)
        path, config = base / 'input.rb', base / 'config.yml'
        config.write_text(case['config_source'])
        for index, parent in enumerate(case['parent_sources']):
            (base / f'parent-{index}.yml').write_text(parent)
        for engine, executable in [('ruby', settings.ruby), ('rust', settings.rust)]:
            path.write_bytes(case['source'].encode())
            command = [str(executable.resolve()), '--config', str(config), '--only', 'Style/TrailingCommaInBlockArgs', '--cache', 'false', '--format', 'json', mode]
            if engine == 'rust':
                command.extend(['--rubocop-version', case['reference_version']])
            process = subprocess.run(command + [str(path)], cwd=root, env=environment, capture_output=True, text=True)
            reason = re.search(r'Unsafe cops cannot have a safe autocorrection \(section [^\n]+\)', process.stderr)
            expected = case['expected'][mode]
            assert process.returncode == expected['exit'], (case['id'], mode, engine, process.stdout, process.stderr)
            assert path.read_bytes().decode() == expected['bytes'], (case['id'], mode, engine, path.read_bytes())
            if expected['exit'] == 2:
                assert reason, (case['id'], engine, process.stderr)
            observations[engine] = {'exit': process.returncode, 'bytes': path.read_bytes().decode(), 'error_reason': reason[0] if reason else None}
        assert observations['ruby'] == observations['rust'], (case['id'], mode, observations)
    return {'id': case['id'], 'mode': mode, 'observations': observations}


with concurrent.futures.ThreadPoolExecutor(max_workers=settings.jobs) as pool:
    rows = list(pool.map(inspect, [(case, mode) for case in cases for mode in ['-a', '-A']]))
settings.output.write_text(json.dumps(rows, indent=2) + '\n')
print('config modes', len(rows), 'all match original exits, bytes and validation reasons')
