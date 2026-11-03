#!/usr/bin/env python3
"""Compare actual CLI/config path scopes using virtual stdin files; never create repo files."""
import argparse
import concurrent.futures
import json
import os
from pathlib import Path
import subprocess

p = argparse.ArgumentParser()
p.add_argument('--ruby-project', required=True, type=Path)
p.add_argument('--native', required=True, type=Path)
p.add_argument('--output', required=True, type=Path)
p.add_argument('--cop', action='append', help='Run only named cops and their auxiliary policy cases')
a = p.parse_args()
model = 'app/models/framework_parity_probe.rb'
service = 'app/services/framework_parity_probe.rb'
spec = 'spec/factories/framework_parity_probe.rb'
rows = [
 ('Performance/TimesMap', '4.times.map { |i| i.to_s }', model, service),
 ('Rails/Presence', 'value.present? ? value : nil', model, service),
 ('Rails/SafeNavigation', 'foo.try!(:bar)', model, service),
 ('Rails/UniqBeforePluck', 'Model.pluck(:name).uniq', model, service),
 ('Rails/RelativeDateConstant', 'DATE = Date.yesterday', model, service),
 ('Rails/LinkToBlank', "link_to 'text', '/url', target: '_blank'", 'app/helpers/framework_parity_probe.rb', model),
 ('Rails/ContentTag', 'tag(:br)', 'app/helpers/framework_parity_probe.rb', model),
 ('Rails/RedundantForeignKey', "class Post; has_many :comments, foreign_key: 'post_id'; end", model, service),
 ('Rails/HasManyOrHasOneDependent', 'class Model; has_many :children; end', model, service),
 ('Rails/InverseOf', 'has_many :children, foreign_key: :owner_id', model, service),
 ('Rails/DynamicFindBy', 'User.find_by_name(name)', model, service),
 ('Rails/RedundantAllowNil', 'validates :name, allow_nil: true, allow_blank: true', model, service),
 ('Rails/RedundantReceiverInWithOptions', 'with_options(flag: true) { |m| m.foo }', model, service),
 ('Rails/LexicallyScopedActionFilter', 'class C; before_action :authorize, only: :missing; end', 'app/controllers/framework_parity_probe.rb', model),
 ('Rails/IndexBy', 'x.map { |el| [el.to_sym, el] }.to_h', model, service),
 ('Rails/IndexWith', 'x.map { |el| [el, el.to_sym] }.to_h', model, service),
 ('Rails/HttpStatus', 'render status: 200', 'app/controllers/framework_parity_probe.rb', model),
 ('Rails/PluralizationGrammar', '2.day', model, service),
 ('Rails/ReflectionClassName', 'has_many :accounts, class_name: Account', model, service),
 ('Rails/Validation', 'validates_presence_of :name', model, service),
 ('Performance/StartWith', r"str.match?(/\Aabc/)", model, service),
 ('Performance/EndWith', r"str.match?(/abc\z/)", model, service),
 ('Performance/DeletePrefix', r"str.gsub(/\Aprefix/, '')", model, service),
 ('Performance/DeleteSuffix', r"str.gsub(/suffix\z/, '')", model, service),
 ('Rails/EnvironmentComparison', "Rails.env == 'production'", model, service),
 ('Rails/UnknownEnv', 'Rails.env.proudction?', model, service),
 ('Rails/Date', 'Date.today', model, service),
 ('Rails/Blank', 'items.nil? || items.empty?', model, service),
 ('Rails/Present', '!items.blank?', model, service),
 ('Rails/OutputSafety', 'input.html_safe', model, service),
 ('Performance/Count', 'items.select { |item| item.active? }.count', model, service),
 ('Performance/Size', '[1].count', model, service),
 ('Performance/RangeInclude', '(1..3).include?(value)', model, service),
 ('Performance/UriDefaultParser', 'URI::Parser.new', model, service),
 ('FactoryBot/FactoryClassName', 'factory :user, class: User', spec, model),
 ('FactoryBot/AttributeDefinedStatically', 'factory :user do; name "Ada"; end', spec, model),
 ('FactoryBot/CreateList', '3.times { create :user }', spec, model),
 ('Performance/FixedSize', '[1, 2].size', model, service),
 ('Performance/FlatMap', 'items.map { |item| [item] }.flatten(1)', model, service),
 ('Rails/ArelStar', 'Model["*"]', model, service),
 ('Rails/FindBy', 'User.where(name: "Ada").take', model, service),
 ('Rails/Pick', 'User.pluck(:id).first', model, service),
 ('Performance/Caller', 'caller.first', model, service),
 ('Performance/InefficientHashSearch', 'options.keys.include?(:name)', model, service),
 ('Performance/ReverseEach', 'items.reverse.each { |item| puts item }', model, service),
 ('Rails/ScopeArgs', 'scope :active, where(active: true)', model, service),
 ('Rails/HasAndBelongsToMany', 'has_and_belongs_to_many :roles', model, service),
 ('Rails/ActiveRecordOverride', 'class User < ApplicationRecord; def save; super; end; end', model, service),
 ('Rails/AssertNot', 'assert !value', 'test/models/framework_parity_test.rb', model),
 ('Performance/BindCall', 'method.bind(object).call(arg)', model, service),
 ('Performance/UnfreezeString', 'String.new("text")', model, service),
 ('Performance/DoubleStartEndWith', 'text = "beta"; text.start_with?("a") || text.start_with?("b")', model, service),
 ('Rails/EnumUniqueness', 'enum :status, [:active, :active]', model, service),
 ('Rails/EnumHash', 'enum :status, [:active, :archived]', model, service),
 ('Rails/SafeNavigationWithBlank', 'perform if value&.blank?', model, service),
 ('Rails/IgnoredSkipActionFilterOption', 'skip_before_action :login, only: :show, if: :trusted?', 'app/controllers/framework_parity_controller.rb', model),
 ('Rails/CreateTableWithTimestamps', 'create_table :users', 'db/migrate/20990101000000_framework_parity.rb', model),
]
cases = [dict(cop=cop, source=source+'\n', path=path, positive=index==0)
         for cop, source, *paths in rows for index, path in enumerate(paths)]
cases += [dict(cop='Rails/CreateTableWithTimestamps', source='create_table :users\n', path=path, positive=False)
          for path in ('db/schema.rb', 'db/migrate/20990101000000_create_active_storage_tables.active_storage.rb')]
cases += [dict(cop='Rails/CreateTableWithTimestamps', source='create_table :users\n', path='db/schema.rb', positive=True, force_exclusion=False),
          dict(cop='Rails/Blank', source='items.nil? || items.empty?\n', path='vendor/framework_parity_probe.rb', positive=False),
          dict(cop='Rails/Blank', source='items.nil? || items.empty?\n', path='vendor/framework_parity_probe.rb', positive=True, force_exclusion=False)]
cases += [dict(cop='Lint/RedundantCopDisableDirective', extra_cops=['Rails/Validation'],
               source='# rubocop:disable Rails/Validation\n'+body+'\n# rubocop:enable Rails/Validation\n',
               path=str(a.ruby_project/model), positive=positive)
          for body,positive in [('validates_presence_of attrs',False),('validates_presence_of :name',False),('validates_presence_of',True),('puts :hello',True)]]
cases += [dict(cop='Rails/LexicallyScopedActionFilter', source='class C; before_action :authorize, only: :missing; end\n', path='app/mailers/framework_parity_probe.rb', positive=True),
          dict(cop='Rails/LexicallyScopedActionFilter', source='class C; before_action :authorize, only: :missing; end # rubocop:disable Rails/LexicallyScopedActionFilter\n', path='app/controllers/framework_parity_probe.rb', positive=False)]
cases += [dict(cop='Rails/RedundantReceiverInWithOptions', source='with_options(flag: true) do |m|\n m.foo # rubocop:disable Rails/RedundantReceiverInWithOptions\n m.bar\nend\n', path=model, positive=True)]
if a.cop:
    selected = set(a.cop)
    known = {case['cop'] for case in cases} | {cop for case in cases for cop in case.get('extra_cops', [])}
    if selected - known:
        p.error('Unknown scope cop: ' + ', '.join(sorted(selected - known)))
    cases = [case for case in cases if case['cop'] in selected or selected.intersection(case.get('extra_cops', []))]
severity = dict(C='convention', W='warning', E='error', F='fatal', R='refactor', I='info')
env = {**os.environ, 'ASDF_RUBY_VERSION':'3.4.9', 'RRUBOCOP_CACHE_DIR':str(a.output/'native-cache'), 'RUBOCOP_CACHE_ROOT':str(a.output/'ruby-cache')}
a.output.mkdir(parents=True, exist_ok=True)
auxiliary_config=a.output/'auxiliary-policy.yml'
auxiliary_config.write_text('inherit_from: '+json.dumps(str(a.ruby_project/'.rubocop.yml'))+'\nLint/RedundantCopDisableDirective:\n  Enabled: true\nRails/Validation:\n  Include:\n    - '+json.dumps(str(a.ruby_project/'app/models/**/*.rb'))+'\n')

def observe(command, case):
    flags = ['--force-exclusion'] if case.get('force_exclusion',True) else []
    selection=['--config',str(auxiliary_config)] if case.get('extra_cops') else ['--only',case['cop']]
    result = subprocess.run(command + ['--stdin', case['path'], '--format', 'json', '--cache', 'false'] + selection + flags, input=case['source'], capture_output=True, text=True, cwd=a.ruby_project, env=env, timeout=30)
    if result.returncode not in (0, 1):
        return dict(error=result.stderr, code=result.returncode)
    output=json.loads(result.stdout)
    offenses=output.get('offenses')
    if offenses is None:
        offenses=[o for file in output.get('files', []) for o in file['offenses']]
    found=[]
    for o in offenses:
        if o['cop_name'] != case['cop']:
            continue
        location=o.get('location')
        line=location['start_line'] if location else o['line']
        column=location['start_column']-1 if location else o['column']
        found.append([line,column,o['message'].removeprefix(case['cop']+': '),severity.get(o['severity'],o['severity'])])
    inspected=output.get('metadata',{}).get('files_inspected',output.get('summary',{}).get('inspected_file_count'))
    return dict(offenses=found,files_inspected=inspected)

def check(case):
    ruby=observe(['bin/rubocop-ruby'],case)
    native=observe([str(a.native)],case)
    return {**case, 'ruby':ruby,'native':native,'matches':'offenses' in ruby and 'offenses' in native and ruby==native}

with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
    results=list(pool.map(check,cases))
failures=[row for row in results if not row['matches']]
missing_positives=[row['cop'] for row in results if row['positive'] and ('offenses' not in row['ruby'] or not row['ruby']['offenses'])]
(a.output/'results.json').write_text(json.dumps(results,indent=2)+'\n')
summary=dict(cases=len(results),production_scope_cases=sum(not c.get('extra_cops') for c in cases),auxiliary_policy_cases=sum(bool(c.get('extra_cops')) for c in cases),rules=len({c['cop'] for c in cases if not c.get('extra_cops')}),catalog_rules=len(rows),auxiliary_rules=sorted({c['cop'] for c in cases}-{r[0] for r in rows}),differences=len(failures),missing_positive_controls=missing_positives)
(a.output/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps(summary,indent=2))
raise SystemExit(bool(failures or missing_positives))
