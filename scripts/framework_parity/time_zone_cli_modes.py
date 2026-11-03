#!/usr/bin/env python3
"""Physical-file CLI modes; every deliberate safety difference remains a failing event."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile

parser=argparse.ArgumentParser()
parser.add_argument('--ruby-project',type=Path,required=True)
parser.add_argument('--native',type=Path,required=True)
parser.add_argument('--output',type=Path,required=True)
args=parser.parse_args()
args.output.mkdir(parents=True,exist_ok=True)
env={**os.environ,'ASDF_RUBY_VERSION':'3.4.9','BUNDLE_GEMFILE':str(args.ruby_project/'Gemfile')}
header="ENV['TZ']='UTC'\nTime.zone='UTC'\n"
cases=[('unused UTC constructor',header+'Time.new\n'),('consumed object type',header+'p Time.new(2020).class.name\n'),('foreign process zone',"ENV['TZ']='EST5'\nTime.zone='UTC'\np Time.new(2020).to_i\n")]
severity=dict(C='convention',W='warning',E='error',F='fatal',R='refactor',I='info')
config="require:\n  - rubocop-rails\nAllCops:\n  DisabledByDefault: true\n  TargetRubyVersion: 3.4\nRails/TimeZone:\n  Enabled: true\n  EnforcedStyle: strict\n"

def run(label,binary,project,flag,pass_number):
    argv=[str(binary),'--config',str(project/'.rubocop.yml'),'--only','Rails/TimeZone','--format','json','--cache','false',flag,'app/models/probe.rb']
    result=subprocess.run(argv,cwd=project,env=env,capture_output=True,text=True,timeout=60)
    (args.output/f'{label}-{pass_number}.stdout.json').write_text(result.stdout)
    (args.output/f'{label}-{pass_number}.stderr').write_text(result.stderr)
    if result.returncode not in (0,1):
        raise RuntimeError(f'{label}: exit{result.returncode}: {result.stderr}')
    output=json.loads(result.stdout)
    offenses=output.get('offenses')
    if offenses is None:
        offenses=[o for file in output['files'] for o in file['offenses']]
    found=[]
    for offense in offenses:
        location=offense.get('location')
        found.append([location['start_line'] if location else offense['line'],location['start_column']-1 if location else offense['column'],offense['message'].removeprefix('Rails/TimeZone: '),severity.get(offense['severity'],offense['severity']),offense['corrected']])
    inspected=output.get('summary',{}).get('inspected_file_count',output.get('metadata',{}).get('files_inspected'))
    return dict(argv=argv,exit=result.returncode,inspected=inspected,offenses=found,source=(project/'app/models/probe.rb').read_text())

results=[]
with tempfile.TemporaryDirectory(prefix='framework-timezone-cli-',dir='/dev/shm') as directory:
    for index,(label,source) in enumerate(cases):
        for flag in ('-a','-A'):
            engines={}
            for name,binary in [('ruby',args.ruby_project/'bin/rubocop-ruby'),('native',args.native)]:
                project=Path(directory)/f'{index}-{name}-{flag}'
                (project/'app/models').mkdir(parents=True)
                (project/'.rubocop.yml').write_text(config)
                (project/'Gemfile.lock').write_bytes((args.ruby_project/'Gemfile.lock').read_bytes())
                (project/'app/models/probe.rb').write_text(source)
                first=run(f'{index}-{flag}-{name}',binary,project,flag,1)
                second=run(f'{index}-{flag}-{name}',binary,project,flag,2)
                engines[name]=dict(first=first,second=second,stable=first['source']==second['source'])
            compared=lambda engine:{key:engines[engine]['first'][key] for key in ('exit','inspected','offenses','source')}
            results.append(dict(label=label,flag=flag,original=source,**engines,matches=compared('ruby')==compared('native')))
summary=dict(cases=len(results),differences=sum(not row['matches'] for row in results),native_idempotent=sum(row['native']['stable'] for row in results),ruby_idempotent=sum(row['ruby']['stable'] for row in results),missing_positive_controls=sum(row['label']=='unused UTC constructor' and row['flag']=='-A' and not any(o[-1] for o in row['native']['first']['offenses']) for row in results))
(args.output/'results.json').write_text(json.dumps(results,indent=2)+'\n')
(args.output/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps(summary,indent=2))
raise SystemExit(bool(summary['differences'] or summary['missing_positive_controls'] or summary['native_idempotent']!=len(results)))
