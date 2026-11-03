#!/usr/bin/env python3
"""Replay the three real Wistia warnings and nonempty controls through actual CLIs."""
import argparse
import json
import os
from pathlib import Path
import subprocess

parser=argparse.ArgumentParser()
parser.add_argument('--ruby-project',type=Path,required=True)
parser.add_argument('--native',type=Path,required=True)
parser.add_argument('--output',type=Path,required=True)
args=parser.parse_args()
args.output.mkdir(parents=True,exist_ok=True)
files=['spec/models/credit_ledger_spec.rb','spec/workers/remix_reminder_worker_spec.rb','spec/lib/live/egress_providers/livekit_tracks_composite_file_provider_spec.rb']
cops='FactoryBot/CreateList,Performance/Count'
controls=[
 ('unbound keyword','Array.new(2) { FactoryBot.create(:project, account:) }',0),
 ('bound keyword','account=1; Array.new(2) { FactoryBot.create(:project, account:) }',1),
 ('unbound times','2.times { create(:project, account:) }',0),
 ('bound times','account=1; 2.times { create(:project, account:) }',1),
 ('numbered predicate','items.select { _1.active? }.count',0),
 ('explicit predicate','items.select { |item| item.active? }.count',1),
 ('nested numbered predicate','items.select { records.any? { _1.active? } }.count',1),
]
severity=dict(C='convention',W='warning',E='error',F='fatal',R='refactor',I='info')
env={**os.environ,'ASDF_RUBY_VERSION':'3.4.9'}

def observe(label,binary,source=None):
    argv=[str(binary),'--only',cops,'--format','json','--cache','false']
    argv+=files if source is None else ['--stdin','spec/models/framework_real_corpus_probe_spec.rb']
    result=subprocess.run(argv,input=source,cwd=args.ruby_project,env=env,capture_output=True,text=True,timeout=90)
    (args.output/(label+'.stdout.json')).write_text(result.stdout)
    (args.output/(label+'.stderr')).write_text(result.stderr)
    if result.returncode not in (0,1):
        raise RuntimeError(f'{label}: exit {result.returncode}: {result.stderr}')
    output=json.loads(result.stdout)
    offenses=output.get('offenses')
    if offenses is None:
        offenses=[dict(offense,path=file['path']) for file in output['files'] for offense in file['offenses']]
    found=[]
    for offense in offenses:
        location=offense.get('location')
        cop=offense['cop_name']
        found.append([offense['path'],location['start_line'] if location else offense['line'],location['start_column']-1 if location else offense['column'],cop,offense['message'].removeprefix(cop+': '),severity.get(offense['severity'],offense['severity'])])
    inspected=output.get('summary',{}).get('inspected_file_count',output.get('metadata',{}).get('files_inspected'))
    return dict(argv=argv,exit=result.returncode,inspected=inspected,offenses=sorted(found))

results=[]
for index,(label,source,expected) in enumerate([('real files',None,0),*controls]):
    ruby=observe(f'{index}-ruby',args.ruby_project/'bin/rubocop-ruby',source)
    native=observe(f'{index}-native',args.native,source)
    matches=ruby['offenses']==native['offenses'] and ruby['inspected']==native['inspected'] and len(ruby['offenses'])==expected
    results.append(dict(label=label,source=source,expected=expected,ruby=ruby,native=native,matches=matches))
summary=dict(cases=len(results),real_files=len(files),controls=len(controls),positive_controls=sum(expected>0 for _,_,expected in controls),differences=sum(not row['matches'] for row in results),wistia_sha=subprocess.check_output(['git','rev-parse','HEAD'],cwd=args.ruby_project,text=True).strip())
(args.output/'results.json').write_text(json.dumps(results,indent=2)+'\n')
(args.output/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps(summary,indent=2))
raise SystemExit(bool(summary['differences']))
