"""Original-source branch coverage for ten existing Rails implementations."""
import json
from pathlib import Path
MINIMUM={'Rails/ApplicationJob','Rails/ApplicationMailer','Rails/ApplicationRecord','Rails/BelongsTo'}
BATCH_ONE={'Rails/ActiveRecordAliases','Rails/ActiveSupportAliases','Rails/ApplicationController','Rails/ApplicationJob','Rails/ApplicationMailer','Rails/ApplicationRecord'}
def cases():
    rows=[]
    def add(cop,source,**config):
        if cop in MINIMUM:config.setdefault('GemVersions',{'railties':'8.1.3.1'})
        if cop=='Rails/RakeEnvironment':config.setdefault('Safe',False)
        path='app/helpers/probe_helper.rb' if cop=='Rails/HelperInstanceVariable' else 'lib/tasks/probe.rake' if cop=='Rails/RakeEnvironment' else 'app/models/probe.rb'
        rows.append(dict(id=f'{cop}:review:{len(rows)}',cop=cop,path=path,source=source if source.endswith('\n') else source+'\n',expected=[],corrected=source,config=config))
    for row in json.loads((Path(__file__).parents[2]/'tests/fixtures/rails-review-upstream/cases.json').read_text()):
        config=dict(row['config'])
        if row['cop'] in MINIMUM and 'TargetRailsVersion' in config:config['GemVersions']={'railties':str(config['TargetRailsVersion'])}
        add(row['cop'],row['source'],**config)
    for source in ('record.update_attributes(name: "Ada")','record&.update_attributes!(name: "Ada")','update_attributes(name: "Ada")','record.update_attributes','record.update_attributes()','update_attributes','record.update_attributes {work}','record.update_attributes(&block)','record.update_attributes(name: "Ada") {work}','record.update_attributes = value','def update_attributes; end','def self.update_attributes; end','alias update_attributes other','undef update_attributes','record.send(:update_attributes, name: "Ada")'):
        add('Rails/ActiveRecordAliases',source)
    for source in ('record.update_attributes(name: "Ada")','record.update_attributes!(name: "Ada")'):
        add('Rails/ActiveRecordAliases',source,AutoCorrect='disabled')
        add('Rails/ActiveRecordAliases',source,Severity='error')
    for receiver in ("'abc'", '"abc"', '%q(abc)', '?a', "('abc')", '\"a#{value}\"', "'a' 'b'", "'a\nb'", '[]', '[1,2]', '%w[a b]', '%i[a b]', 'items', ':abc'):
        for method in ('starts_with?','ends_with?','append','prepend'):
            for args in ('', "'a'", "'a','b'", '*values', '&block', '**options', 'key: 1'):
                for operator in ('.','&.'):
                    add('Rails/ActiveSupportAliases',f'{receiver}{operator}{method}({args})')
    for source in ("'abc'.starts_with?('a')", "[].prepend(1)", "[].append(1)", "'abc'.starts_with?(\n 'a' # keep\n)", "'abc'.starts_with?('a') {work}"):
        add('Rails/ActiveSupportAliases',source)
        add('Rails/ActiveSupportAliases',source,AutoCorrect='disabled')
        add('Rails/ActiveSupportAliases',source,Severity='warning')
    bases={'ApplicationController':'ActionController::Base','ApplicationJob':'ActiveJob::Base','ApplicationMailer':'ActionMailer::Base','ApplicationRecord':'ActiveRecord::Base'}
    for preferred,base in bases.items():
        cop='Rails/'+preferred
        for name in ('Example',preferred,'Nested::Example','Nested::'+preferred,'::'+preferred):
            for superclass in (base,'::'+base,'Other::'+base,preferred):
                add(cop,f'class {name} < {superclass}; end')
        for source in (f'Class.new({base})',f'::Class.new(::{base})',f'Other::Class.new({base})',f'Class&.new({base})',f'Class.new({base},extra)',f'Class.new({base},&block)',f'Class.new({base}) {{work}}',f'{preferred} = Class.new({base})',f'{preferred} = ::Class.new({base}) {{work}}',f'::{preferred} = Class.new({base})',f'Other::{preferred} = Class.new({base})',f'{preferred} = (Class.new({base}))',f'{preferred} = Class.new({base}).freeze',f'{preferred} ||= Class.new({base})'):
            add(cop,source)
        for gems in ({},{'railties':'4.2.11.3'},{'railties':'5.0.0'},{'railties':'8.1.3.1'}):
            for version in (4.2,5.0,8.1):
                add(cop,f'class Example < {base}; end',GemVersions=gems,TargetRailsVersion=version)
        add(cop,f'class Example < {base}; end',AutoCorrect='disabled')
        add(cop,f'Class.new({base})',Severity='warning')
        add(cop,f'class Example < {base.replace("::", "::\n # keep namespace\n ")}; end')
    for receiver in ('','self.','object.','object&.'):
        for name in (':blog',"'blog'",'name'):
            for options in ('required: true','required: false',':required => true',"'required' => true",'"required": false','required: nil','required: (true)','required: false,optional: false','required: nil,required: true','required: false,required: true','other: {required: false}'):
                add('Rails/BelongsTo',f'{receiver}belongs_to {name}, {options}')
    for source in ('belongs_to required: false','belongs_to :x, {required: false}, other','belongs_to :x, {required: false}, &block','belongs_to :x, required: false do; work; end','belongs_to :x, required: # keep\n false'):
        add('Rails/BelongsTo',source)
    for gems in ({},{'railties':'4.2.11.3'},{'railties':'5.0.0'},{'railties':'8.1.3.1'}):
        for version in (4.2,5.0,8.1):add('Rails/BelongsTo','belongs_to :x, required: false',GemVersions=gems,TargetRailsVersion=version)
    for body in ('@item','@item = value','@item ||= value','@item &&= value','@item += value','@item ||= @other','@item = (@other ||= value)','@item = "||="','@item,@other = values','for @item in values; end','defined?(@item)','@item.call','@item.name=value'):
        for declaration in ('module Helper','class Helper','class Helper < ActionView::Helpers::FormBuilder','class Helper < ::ActionView::Helpers::FormBuilder','class Helper < Other::ActionView::Helpers::FormBuilder','class Helper < FormBuilder'):
            add('Rails/HelperInstanceVariable',f'{declaration}; def example; {body}; end; end')
    add('Rails/HelperInstanceVariable','class Outer < ActionView::Helpers::FormBuilder; class Inner; @item; end; end')
    add('Rails/HelperInstanceVariable','class Outer; class Inner < ActionView::Helpers::FormBuilder; @item; end; @other; end')
    for source in ("task :foo do; work; end", "task 'foo' do; work; end", "task :default do; work; end", "task %s(default) do; work; end", "task :'foo:bar' do; work; end", "task :'foo|bar' do; work; end", "task :'foo bar' do; work; end", "task :[] do; work; end", "task name do; work; end", "task do; work; end", "task nil do; work; end", "task :foo, [:name] do; work; end", "task :foo, %w[name] do; work; end", "task :foo, [:name]=>[] do; work; end", "task foo: [] do; work; end", "task foo: [:other] do; work; end", "task foo: nil do; work; end", "task foo: false do; work; end", "task foo: %w[] do; work; end", "task foo: %i[other] do; work; end", "task foo: [# keep\n] do; work; end", "task foo: [],other: :dependency do; work; end", "task default: [],foo: [] do; work; end", "task :foo", "object.task :foo do; work; end", "task(:foo) { _1 }", "task(:foo) { |x| x }", "task :foo, # keep\n [:name] do; work; end"):
        add('Rails/RakeEnvironment',source)
    for ruby in (2.5,2.7,3.4):add('Rails/RakeEnvironment','task(:foo) { _1 }',TargetRubyVersion=ruby)
    for receiver in ('','self.','other.','other&.'):
        for call in ("read_attribute(:name)","read_attribute('name')",'read_attribute(name)','read_attribute(:a,:b)', 'read_attribute()', 'write_attribute(:name,value)', 'write_attribute(:name)', 'write_attribute(:a,value,extra)', 'write_attribute(:name,key: 1)'):
            add('Rails/ReadWriteAttribute',receiver+call)
    for method in ('name','name=','other'):
        for argument in (':name',"'name'",'%s(name)','%q(name)','"na\\u006de"','name',"('name')", "'na' 'me'", 'nil', '1'):
            for verb in ('read_attribute','write_attribute'):
                call=f'{verb}({argument}'+(',value' if verb=='write_attribute' else '')+')'
                for owner in ('','self.'):
                    add('Rails/ReadWriteAttribute',f'def {owner}{method}(value=nil); {call}; end')
    for expression in ('write_attribute(:name,1).to_s','write_attribute(:name,1) || other','other || write_attribute(:name,1)','write_attribute(:name,1) && other','other && write_attribute(:name,1)','write_attribute(:name,1) and other','other and write_attribute(:name,1)','!write_attribute(:name,1)','1 + write_attribute(:name,1)','consume(write_attribute(:name,1))','result=write_attribute(:name,1)','read_attribute( # keep\n:name)','read_attribute(:name) {work}','write_attribute(:name,1) {work}'):
        add('Rails/ReadWriteAttribute',expression)
    for cop,source in (('Rails/BelongsTo','belongs_to :x, required: false'),('Rails/HelperInstanceVariable','@value'),('Rails/RakeEnvironment','task :foo do; work; end'),('Rails/ReadWriteAttribute','read_attribute(:name)')):
        add(cop,source,Severity='warning')
        add(cop,source,AutoCorrect='disabled')
    profiles={
        'Rails/ActiveRecordAliases':'record.update_attributes(name: "Ada")',
        'Rails/ActiveSupportAliases':"'abc'.starts_with?('a')",
        'Rails/ApplicationController':'class Example < ActionController::Base; end',
        'Rails/ApplicationJob':'class Example < ActiveJob::Base; end',
        'Rails/ApplicationMailer':'class Example < ActionMailer::Base; end',
        'Rails/ApplicationRecord':'class Example < ActiveRecord::Base; end',
        'Rails/BelongsTo':'belongs_to :blog, required: false',
        'Rails/HelperInstanceVariable':'@value',
        'Rails/RakeEnvironment':'task :probe do; work; end',
        'Rails/ReadWriteAttribute':'read_attribute(:name)',
    }
    for cop,source in profiles.items():
        for safe in (False,True):
            for correction in (False,True):add(cop,source,Safe=safe,SafeAutoCorrect=correction)
    from rails_review_runtime import programs
    for cop,source,config in programs():
        add(cop,source,**config)
        rows[-1]['trusted_runtime']=True
    return rows
