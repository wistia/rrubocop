#!/usr/bin/env python3
"""Check the original requires_gem gate through real files and both CLIs."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--ruby-project',type=Path,required=True)
p.add_argument('--native',type=Path,required=True)
p.add_argument('--output',type=Path,required=True)
a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True)
rows=[]
with tempfile.TemporaryDirectory(prefix='rrubocop-final-rails-lock-',dir='/dev/shm') as directory:
    root=Path(directory);(root/'spec/requests').mkdir(parents=True)
    source=root/'spec/requests/probe_spec.rb';source.write_text('get :show, id: 1\n')
    config=root/'.rubocop.yml'
    env={**os.environ,'ASDF_RUBY_VERSION':'3.4.9','BUNDLE_GEMFILE':str(a.ruby_project/'Gemfile')}
    for target in (4.2,8.1):
        config.write_text(f'require: [rubocop-rails]\nAllCops:\n  TargetRubyVersion: 3.4\n  TargetRailsVersion: {target}\n  DisabledByDefault: true\nRails/HttpPositionalArguments:\n  Enabled: true\n')
        for version in (None,'4.2.11.3','5.0.0','8.1.3.1'):
            lock=root/'Gemfile.lock'
            if version is None:lock.unlink(missing_ok=True)
            else:lock.write_text(f'GEM\n  remote: https://rubygems.org/\n  specs:\n    railties ({version})\n\nPLATFORMS\n  ruby\n\nDEPENDENCIES\n  railties\n')
            observations=[]
            for command in ([str(a.ruby_project/'bin/rubocop-ruby')],[str(a.native)]):
                run=subprocess.run(command+['--config',str(config),'--only','Rails/HttpPositionalArguments','--format','json','--cache','false',str(source)],cwd=root,env=env,capture_output=True,text=True,timeout=30)
                if run.returncode not in (0,1):raise RuntimeError(run.stderr)
                value=json.loads(run.stdout);offenses=value.get('offenses')
                if offenses is None:offenses=[o for file in value.get('files',[]) for o in file['offenses']]
                observations.append(sum(o['cop_name']=='Rails/HttpPositionalArguments' for o in offenses))
            expected=int(target>=5.0 and version is not None and not version.startswith('4.'))
            rows.append(dict(target=target,railties=version,ruby=observations[0],native=observations[1],expected=expected,matches=observations==[expected,expected]))
(a.output/'results.json').write_text(json.dumps(rows,indent=2)+'\n')
summary=dict(controls=len(rows),differences=sum(not r['matches'] for r in rows));(a.output/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps(summary));raise SystemExit(bool(summary['differences']))
