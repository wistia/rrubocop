#!/usr/bin/env python3
import argparse, json, pathlib, subprocess, tempfile
p=argparse.ArgumentParser();p.add_argument('before');p.add_argument('after');p.add_argument('--output',required=True);args=p.parse_args()
cases=[]
def add(cop, source, variants=({},)):
 for config in variants:
  cases.append((cop,source,config))
add('Naming/VariableName','Foo = 1\n@badName = 2\ndef f(badName)\n localBad = badName\nend\n', ({'AllowedPatterns':['^badName$','['],'ForbiddenPatterns':['Bad$','^bad','[']},{'EnforcedStyle':'camelCase','AllowedIdentifiers':['localBad']},{}))
add('Layout/SpaceAroundOperators','a    = 1\nbb   = 2\n\n# aligned\nx=2\ny = 3+4\nvalues = { :a=>1, :b =>2 }\n', ({'AllowForAlignment':True},{'AllowForAlignment':False}))
add('Layout/SpaceAroundOperators','é  = 1\nx  = 2\narray = [1,2]\n\nx <<= 2\nxx  = 3\n'.replace('\n','\r\n'))
add('Style/HashSyntax','h = { :a => 1, :b? => :value, "str" => 3 }\nreturn :a => 1, :b => 2\n', ({'EnforcedStyle':'ruby19'},{'EnforcedStyle':'no_mixed_keys'},{'EnforcedStyle':'hash_rockets'},{'UseHashRocketsWithSymbolValues':True},{'PreferHashRocketsForNonAlnumEndingSymbols':True}))
add('Style/HashSyntax','foo = 1\nbar = 2\ndef f(foo, bar)\n call foo: foo, bar: bar\n {foo: foo, bar: bar}\nend\n',tuple({'EnforcedShorthandSyntax':v,'TargetRubyVersion':3.4} for v in ['always','never','consistent','either_consistent']))
add('Layout/SpaceBeforeFirstArg','foo  bar\n#    word\n\nfoo    baz\n       word\n'.replace('\n','\r\n'),({'AllowForAlignment':True},{'AllowForAlignment':False}))
for cop in ['Metrics/ClassLength','Metrics/ModuleLength']:
 add(cop,'module A\n # comment\n class B\n  def f; 1; end\n end\n module C\n  X = 1\n end\n class << self\n  def g; 2; end\n end\n def h; 3; end\nend\n',({'Max':1,'CountComments':False},{'Max':1,'CountComments':True}))
for cop in ['Style/TrailingCommaInArrayLiteral','Style/TrailingCommaInHashLiteral']:
 add(cop,'a = [\n [1,\n 2,\n ],\n {foo: [3,\n 4,\n ],},\n]\nb = {\n text: <<~TXT,\n   hello\n TXT\n tail: 2,\n}\n',tuple({'EnforcedStyleForMultiline':s} for s in ['no_comma','comma','consistent_comma']))
add('Style/TrailingCommaInArguments','outer(\n inner(\n  1,\n ),\n)\ncall(\n <<~TEXT,\n   hi\n TEXT\n)\n', tuple({'EnforcedStyleForMultiline':s} for s in ['no_comma','comma','consistent_comma']))
for cop in ['Layout/MultilineOperationIndentation','Layout/IndentationWidth']:
 add(cop,'if true\n  x = 1 +\n    2\n  y =\n      3\n  call do\n     value\n  end\nend\n'.replace('\n','\r\n'),({'Width':2},{'Width':4}))
failures=[];count=0; raw_order_differences=0; positive_modes=0; corrected_modes=0
with tempfile.TemporaryDirectory(prefix='rrubocop-core-controls-') as directory:
 root=pathlib.Path(directory);sourcepath=root/'probe.rb';configpath=root/'config.yml'
 for index,(cop,source,options) in enumerate(cases):
  config={'AllCops':{'NewCops':'enable','TargetRubyVersion':3.4},cop:{'Enabled':True,**options}}
  configpath.write_text(json.dumps(config))
  for mode in [[],['-a'],['-A']]:
   observations=[]
   for binary in [args.before,args.after]:
    sourcepath.write_text(source)
    proc=subprocess.run([binary,'--config',str(configpath),'--rubocop-version','1.65.1','--only',cop,'--cache','false','--format','json','--stdin',str(sourcepath),*mode],input=source.encode(),capture_output=True,cwd=directory)
    try: payload=json.loads(proc.stdout)
    except Exception: payload=proc.stdout.decode(errors='replace')
    observations.append((proc.returncode,payload,sourcepath.read_bytes(),proc.stderr))
   count+=1
   if isinstance(observations[0][1],dict):
    positive_modes += bool(observations[0][1].get('offenses'))
    corrected_modes += observations[0][2] != source.encode()
    raw_order_differences += observations[0][1] != observations[1][1]
    for observation in observations:
     if isinstance(observation[1],dict):
      for key in ['offenses','files']:
       if isinstance(observation[1].get(key),list):observation[1][key].sort(key=lambda value:json.dumps(value,sort_keys=True))
   if observations[0]!=observations[1]:failures.append({'case':index,'cop':cop,'mode':mode,'before':repr(observations[0]),'after':repr(observations[1])})
result={'before':args.before,'after':args.after,'program_config_cases':len(cases),'public_mode_comparisons':count,'positive_modes':positive_modes,'corrected_modes':corrected_modes,'raw_json_order_differences':raw_order_differences,'differences':len(failures),'failures':failures,'scope':'Frozen native baseline vs optimized native; normalized JSON (offense order sorted), exit, stderr, corrected file bytes. Not a fresh Ruby oracle.'}
pathlib.Path(args.output).write_text(json.dumps(result,indent=2));print(json.dumps({k:v for k,v in result.items() if k!='failures'}));raise SystemExit(bool(failures))
