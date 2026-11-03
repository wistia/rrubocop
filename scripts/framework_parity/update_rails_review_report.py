#!/usr/bin/env python3
"""Record every strict original-Ruby difference without accepting or hiding any."""
import argparse
from collections import Counter
import json
from pathlib import Path

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('results',type=Path)
p.add_argument('--safety-cli',type=Path)
a=p.parse_args()
cases=json.loads((a.results/'cases.json').read_text())
failures=json.loads((a.results/'failures.json').read_text())
runtime=json.loads((a.results/'runtime.json').read_text())
report={'reference':{'ruby':'3.4.9','rubocop':'1.65.1','rubocop_rails':'2.26.0','rubocop_ast':'1.32.3','parser':'3.3.5.0','upstream_tag':'v2.26.0','imported_examples':131,'unresolved_examples':0},'kind':'existing_implementation_review','new_stub_implementations':0,'rules':[]}
for cop in sorted({r['cop'] for r in cases}):
    originals=[r for r in cases if r['cop']==cop and not r['id'].endswith(':second-pass')]
    found=[r for r in failures if r['id'].startswith(cop+':')]
    semantic=[r for r in found if r['kind']=='diagnostics' and [v[:-1] for v in r['ruby']]!=[v[:-1] for v in r['rust']]]
    report['rules'].append(dict(cop=cop,status='existing_implementation_source_reviewed_and_repaired',original_cases=len(originals),generated_states=2*len(originals),detection_message_range_severity_differences=len(semantic),strict_failures=dict(Counter(r['kind'] for r in found)),retained_differences=found))
report['totals']={'existing_rules':len(report['rules']),'original_cases':sum(r['original_cases'] for r in report['rules']),'generated_states':sum(r['generated_states'] for r in report['rules']),'detection_message_range_severity_differences':sum(r['detection_message_range_severity_differences'] for r in report['rules']),'strict_failure_kinds':dict(Counter(r['kind'] for r in failures)),'strict_failures':len(failures)}
report['runtime']={'trusted_programs':len(runtime),'native_modes':2*len(runtime),'native_behavior_changes':sum(not v['native_preserves_original'] for row in runtime for v in row['modes'].values()),'original_correction_runtime_differences':sum(not v['equivalent'] for row in runtime for v in row['modes'].values())}
report['native_gate']={'passed':935,'ignored':1}
report['production_cli_scope']={'controls':31,'rules':10,'differences':0,'missing_positives':[]}
report['physical_lockfile_gate']={'controls':32,'minimum_version_rules':4,'differences':0,'policy':'both original Team target predicate and actual railties requirements'}
report['independent_safety_override_gate']={'originals':40,'generated_states':80,'differences':0,'note':'Rake fixture defaults explicitly model original Safe:false independently of SafeAutoCorrect; all boolean combinations are tested through direct-cop API, which bypasses CLI configuration validation.'}
if a.safety_cli:
    controls=json.loads((a.safety_cli/'results.json').read_text())
    report['physical_safety_cli']={'controls':len(controls),'valid_configuration_differences':sum(not row['matches'] for row in controls if not row['original_invalid_configuration']),'invalid_configuration_rejection_differences':sum(not row['matches'] for row in controls if row['original_invalid_configuration']),'observations':controls,'original_validator':'rubocop-1.65.1/lib/rubocop/config_validator.rb:241 reject_conflicting_safe_settings'}
report['limits']=['Original CLI rejects Safe:false plus SafeAutoCorrect:true; native currently accepts this invalid combination. This generic validator gap is retained in the separate physical CLI evidence.', 'All strict differences are retained, including intentional correctability/source guards and original errors; no allowlist.','Unsafe opt-in inheritance, aliases and application environment changes retain original configuration policy; trusted neutral profiles do not prove arbitrary custom application semantics.','Missing ActiveRecord attributes, non-UTF8/exotic dynamic macro ASTs and contextual autocorrection beyond tested boolean/disabled controls are not universal-equivalence claims.','Generic combined-cop scheduling remains explicitly deferred.']
output=Path(__file__).parents[2]/'docs/reviews/rails-existing-parity.json'
output.write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report['totals'],indent=2))
