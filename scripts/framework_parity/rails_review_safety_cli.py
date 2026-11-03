#!/usr/bin/env python3
"""Prove independent Rake safety options through real CLI config and source edits."""
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
with tempfile.TemporaryDirectory(prefix='rrubocop-rake-safety-',dir='/dev/shm') as directory:
    root=Path(directory);source=root/'probe.rake';config=root/'.rubocop.yml'
    env={**os.environ,'ASDF_RUBY_VERSION':'3.4.9','BUNDLE_GEMFILE':str(a.ruby_project/'Gemfile')}
    before='task :probe do; work; end\n';after='task probe: :environment do; work; end\n'
    for safe in (False,True):
        for correction in (False,True):
            config.write_text(f'require: [rubocop-rails]\nAllCops:\n  TargetRubyVersion: 3.4\n  DisabledByDefault: true\nRails/RakeEnvironment:\n  Enabled: true\n  Safe: {str(safe).lower()}\n  SafeAutoCorrect: {str(correction).lower()}\n')
            observations=[]
            for command in ([str(a.ruby_project/'bin/rubocop-ruby')],[str(a.native)]):
                source.write_text(before)
                run=subprocess.run(command+['--config',str(config),'--only','Rails/RakeEnvironment','--cache','false','--format','json','-a',str(source)],cwd=root,env=env,capture_output=True,text=True,timeout=30)
                if run.returncode in (0,1):json.loads(run.stdout)
                observations.append(dict(code=run.returncode,source=source.read_text(),error=run.stderr if run.returncode not in (0,1) else None))
            invalid=not safe and correction
            expected=after if safe and correction else before
            matches=all((o["code"]==2 if invalid else o["code"] in (0,1)) and o["source"]==expected for o in observations)
            rows.append(dict(safe=safe,safe_autocorrect=correction,original_invalid_configuration=invalid,ruby=observations[0],native=observations[1],expected=expected,matches=matches))
(a.output/'results.json').write_text(json.dumps(rows,indent=2)+'\n')
summary=dict(controls=len(rows),differences=sum(not r['matches'] for r in rows));(a.output/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps(summary));raise SystemExit(bool(summary['differences']))
