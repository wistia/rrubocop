import argparse, pathlib, tempfile, subprocess, json, os
parser = argparse.ArgumentParser(description='Compare alignment correction metadata without executing specimen programs.')
parser.add_argument('--ruby', type=pathlib.Path, required=True)
parser.add_argument('--rust', type=pathlib.Path, required=True)
parser.add_argument('--gemfile', type=pathlib.Path, required=True)
parser.add_argument('--output', type=pathlib.Path, required=True)
parser.add_argument('--cop', choices=['Layout/ArrayAlignment', 'Layout/ParameterAlignment'], default='Layout/ArrayAlignment')
parser.add_argument('--width-controls-only', action='store_true')
settings = parser.parse_args()
rows = []
root = settings.gemfile.resolve().parent
env = dict(os.environ, BUNDLE_GEMFILE=str(settings.gemfile.resolve()))
source = 'a = [one,\n two]\n' if settings.cop == 'Layout/ArrayAlignment' else 'def foo(one,\n two)\nend\n'
corrected = source.replace('\n two', '\n  two')
controls = [('detect', {}, None, False)]
for option, value in [('AutoCorrect', 'disabled'), ('AutoCorrect', False), ('SafeAutoCorrect', False), ('Safe', False)]:
    for mode in ['-a', '-A']:
        controls.append((str((option, value, mode)), {option: value}, mode, mode == '-A' and option != 'AutoCorrect'))
if settings.width_controls_only:
    controls = []
    for label, own, global_width, expected_width in [('missing', None, 4, 4), ('null', {'IndentationWidth': None}, 4, 4), ('own-two', {'IndentationWidth': 2}, 4, 2), ('own-zero', {'IndentationWidth': 0}, 4, 0), ('global-zero', None, 0, 0), ('both-null', None, None, 2)]:
        for mode in ['-a', '-A']:
            controls.append((label + '-' + mode, own or {}, mode, True, global_width, expected_width))
with tempfile.TemporaryDirectory() as directory:
    base = pathlib.Path(directory)
    path = base / 'input.rb'
    config = base / 'config.yml'
    for control in controls:
        label, options, mode, changes = control[:4]
        specific = {'Enabled': True, 'EnforcedStyle': 'with_fixed_indentation'} | ({} if settings.width_controls_only else {'IndentationWidth': 2}) | options
        sibling = {'Layout/IndentationWidth': {'Width': control[4]}} if settings.width_controls_only else {}
        expected = source.replace('\n two', '\n' + ' ' * control[5] + 'two') if settings.width_controls_only else corrected
        config.write_text(json.dumps({'AllCops': {'DisabledByDefault': True, 'NewCops': 'disable', 'TargetRubyVersion': 3.4}, settings.cop: specific} | sibling))
        observed = []
        for name, exe in [('ruby', str(settings.ruby.resolve())), ('rust', str(settings.rust.resolve()))]:
            path.write_text(source)
            cmd = [exe, '--config', str(config), '--only', settings.cop, '--cache', 'false', '--format', 'json'] + ([mode] if mode else []) + (['--rubocop-version', '1.65.1'] if name == 'rust' else []) + [str(path)]
            process = subprocess.run(cmd, cwd=root, env=env, capture_output=True, text=True)
            assert process.returncode == (0 if changes else 1), (label, name, process.stdout, process.stderr)
            assert path.read_text() == (expected if changes else source), (label, name, path.read_text())
            report = json.loads(process.stdout)
            offenses = report['files'][0]['offenses'] if 'files' in report else report['offenses']
            assert len(offenses) == 1, (label, name, report)
            assert offenses[0]['corrected'] == changes, (label, name, offenses)
            observed.append({'engine': name, 'exit': process.returncode, 'corrected': offenses[0]['corrected'], 'bytes': path.read_text()})
        rows.append({'case': label, 'observations': observed})
settings.output.write_text(json.dumps(rows, indent=2) + '\n')
print('controls', len(rows), 'all match')
