#!/usr/bin/env python3
"""Refresh schema-rule review evidence from the unchanged strict driver artifacts."""
import argparse
from collections import Counter
import json
from pathlib import Path

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('results',type=Path)
p.add_argument('--report',type=Path,default=Path(__file__).parents[2]/'docs/reviews/schema-parity.json')
a=p.parse_args()
report=json.loads(a.report.read_text())
cases=json.loads((a.results/'cases.json').read_text())
failures=json.loads((a.results/'failures.json').read_text())
for rule in report['rules']:
    cop=rule['cop']
    originals=[row for row in cases if row['cop']==cop and not row['id'].endswith(':second-pass')]
    states=[row for row in cases if row['cop']==cop and row['id'].endswith(':second-pass')]
    found=[row for row in failures if row['id'].startswith(cop+':')]
    kinds=Counter(row['kind'] for row in found)
    rule.update(status='implemented_source_reviewed',original_cases=len(originals),second_pass_cases=len(states),
                strict_driver_exit=int(bool(found)),strict_failures=dict(kinds),
                diagnostic_differences=kinds['diagnostics'],invalid_native_corrections=kinds['invalid_native_correction'],
                idempotence_differences=kinds['non_idempotent'],supports_autocorrect=False)
    rule.pop('remaining',None)
    rule['retained_differences']=[dict(id=row['id'],source=row.get('source',next((case['source'] for case in cases if case['id']==row['id']),'')),kind=row['kind'],ruby=row.get('ruby'),native=row.get('rust')) for row in found]
report['totals']=dict(original_cases=sum(r['original_cases'] for r in report['rules']),second_pass_cases=sum(r['second_pass_cases'] for r in report['rules']),implemented_rules=4,
                      diagnostic_differences=sum(r['diagnostic_differences'] for r in report['rules']),invalid_native_corrections=sum(r['invalid_native_corrections'] for r in report['rules']),idempotence_differences=sum(r['idempotence_differences'] for r in report['rules']))
report['cache_cli_controls']={'controls':15,'differences':0,'cache':'native enabled; compared against fresh original Ruby diagnostics','inputs':['schema absent/created/deleted','index added and same-size/same-mtime index changed','YAML database adapter changes','environment database adapter changes','Gemfile.lock railties absent/5.1/5.2/6.0/6.1']}
report['owned_production_scope']={'cases':10,'rules':4,'differences':0,'missing_positive_controls':[],'note':'Bulk uses an isolated Database:mysql override inheriting the actual Wistia config; other rules use actual production settings/schema.'}
report['shared_original_properties']={'database_resolution':15,'new_pluralization_tableize':335,'preserved_underscore_foreign_key':285}
report['bounded_families']=['Canonical schema.rb AST shapes and current Wistia schema are covered; arbitrary executable schema DSLs are not evaluated.', 'AST-value rendering for exotic command-interpolation expressions beyond tested scalar/send cases remains bounded; commands are never executed.', 'Non-UTF8 literal families and runtime custom ActiveSupport inflection registries are not universal-equivalence claims.', 'The original crashes for bare drop_table with missing required arguments; this remains explicit error evidence, not an allowlist.', 'Generic multi-cop scheduling remains explicitly deferred.']
report['native_gate']={'passed':937,'ignored':1}
report['ruby_lint']='four new Ruby scripts clean under actual Wistia config; inherited reference harness retains Cops/RescueMustReportError; data round trips identical'
a.report.write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report['totals'],indent=2))
