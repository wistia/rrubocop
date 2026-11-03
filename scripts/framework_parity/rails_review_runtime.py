"""Authored observations only; upstream specs and imported task bodies are never run."""
def programs():
    rows=[]
    for call in ('record.update_attributes(name: value)', 'record&.update_attributes!(name: value)', 'nil&.update_attributes(name: value)'):
        source='''$events=[]
def value; $events << :argument; 'Ada'; end
class Record
attr_reader :attributes
def initialize; @attributes={}; end
def update(**attributes); $events << :update; @attributes.merge!(attributes); true; end
alias update_attributes update
alias update! update
alias update_attributes! update!
end
record=Record.new
p '''+call+'\np record.attributes\np $events\n'
        rows.append(('Rails/ActiveRecordAliases',source,{}))
    for call in ("'abc'.starts_with?('a')", "'abc'&.ends_with?('c')", "[1,2].prepend(3)", "[1,2]&.append(3)"):
        rows.append(('Rails/ActiveSupportAliases',"require 'active_support/core_ext/string/starts_ends_with'\np "+call+'\n',{}))
    bases={'ApplicationController':('ActionController','Controllers'),'ApplicationJob':('ActiveJob','Jobs'),'ApplicationMailer':('ActionMailer','Mailers'),'ApplicationRecord':('ActiveRecord','Models')}
    for preferred,(namespace,_) in bases.items():
        source=f'module {namespace}; class Base; def marker; :same; end; end; end\nclass {preferred} < {namespace}::Base; end\nclass Example < {namespace}::Base; end\np Example.new.marker\n'
        rows.append(('Rails/'+preferred,source,{}))
    prelude='''$events=[]
def belongs_to(name, **options)
required=options.key?(:required) ? options[:required] : !options.fetch(:optional, false)
$events << [name,required]
end
'''
    for call in ('belongs_to :blog, required: false', 'belongs_to :blog, required: true', 'belongs_to :blog, required: false, optional: false', 'belongs_to :blog, optional: false, required: false'):
        rows.append(('Rails/BelongsTo',prelude+call+'\np $events\n',{}))
    prelude="require 'rake'\nextend Rake::DSL\n$events=[]\nRake::Task.define_task(:environment)\n"
    for definition,invoke in (("task :probe do; $events << :run; end", "Rake::Task[:probe].invoke"),("task :probe, [:name] do |task,args|; $events << args[:name]; end", "Rake::Task[:probe].invoke('Ada')"),("task :'foo:bar' do; $events << :run; end", "Rake::Task['foo:bar'].invoke")):
        rows.append(('Rails/RakeEnvironment',prelude+definition+'\n'+invoke+'\np $events\n',{}))
    prelude='''class Record
attr_reader :events, :attributes
def initialize; @events=[]; @attributes={name: 1}; end
def read_attribute(key); @events << [:read,key]; @attributes[key]; end
alias [] read_attribute
def write_attribute(key,value); @events << [:write,key,value]; @attributes[key]=value; end
alias []= write_attribute
def run
'''
    for expression in ('read_attribute(:name)','write_attribute(:name,2)','write_attribute(:name,2).to_s','write_attribute(:name,nil) || :fallback','false || write_attribute(:name,2)','write_attribute(:name,2) and :next','!write_attribute(:name,2)','1 + write_attribute(:name,2)'):
        source=prelude+expression+'\nend\nend\nrecord=Record.new\np record.run\np record.attributes\np record.events\n'
        rows.append(('Rails/ReadWriteAttribute',source,{}))
    return rows
