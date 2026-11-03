"""Original schema/database rule fixtures; imports are parsed without execution."""
import json
from pathlib import Path

COPS = ('Rails/BulkChangeTable','Rails/NotNullColumn','Rails/ReversibleMigration','Rails/UniqueValidationWithoutIndex')

def extend(record, rows):
    counts = {}
    def add(cop, source, **config):
        if cop in ('Rails/ReversibleMigration','Rails/BulkChangeTable'):
            config.setdefault('TargetRailsVersion',8.1)
        record(cop, source, **config)
        counts[cop] = counts.get(cop,0) + 1
        rows[-1]['id'] = f'{cop}:schema:{counts[cop]}'
        rows[-1]['path'] = 'app/models/schema_parity.rb' if cop.endswith('WithoutIndex') else 'db/migrate/20990101000000_schema_parity.rb'
    for row in json.loads((Path(__file__).parents[2]/'tests/fixtures/schema-upstream/cases.json').read_text()):
        add(row['cop'],row['source'].rstrip('\n'),**row.get('config',{}))
    for options in ('null: false','null: false, default:nil','null:false, default:false','null:false, default:0','null:false, default:unknown','null: nil','null:false, **options','null:true,null:false'):
        add('Rails/NotNullColumn', f'add_column :users,:name,:string,{options}')
    for database in ('mysql','postgresql','sqlite',None,False):
        for type in (':text',"'text'",':virtual',"'virtual'",'type'):
            add('Rails/NotNullColumn',f'add_column :users,:name,{type},null:false',Database=database)
    for source in ('change_table(:users) { |t| t.string :name,null:false }','change_table(:users) { |t| t.column :name,:string,null:false }','change_table(:users) { |t| t.add_reference :users,:name,null:false }','change_table(:users) { |t| begin; t.string :name,null:false; end }','change_table(:users) { |t| if flag; t.string :name,null:false; end }','change_table(:users) { _1.string :name,null:false }','self.add_column :users,:name,:string,null:false','add_reference :users,:name,null:false','add_column :users,:name,:string,null:false do; work; end'):
        add('Rails/NotNullColumn',source)
    add('Rails/NotNullColumn','add_column :users,:name,:string,null:false',AutoCorrect='disabled')

    for body in ('(t.string :name,null:false)','((t.string :name,null:false))','begin; t.string :name,null:false; end','(t.string :name,null:false); marker','marker; t.string :name,null:false','t.string :name,"null": false','t.string :name,"null" => false','t.string :name,null:(false)','t.string :name,null:false,default:(nil)','t.column :name,:virtual,null:false','t.virtual :name,null:false','t.string :name,null:false do; work; end'):
        add('Rails/NotNullColumn', f'change_table(:users) {{ |t| {body} }}')
    add('Rails/NotNullColumn','add_column :users,:name,:string,null:false',Severity='warning')

    for method in ('text','virtual','string'):
        add('Rails/NotNullColumn',f'change_table(:users) {{ |t| t.{method} :name,null:false }}',Database='mysql')

    migration = 'class ExampleMigration < ActiveRecord::Migration[7.0]; def change; %s; end; end'
    for operation in ('execute', 'execute("SQL")', 'obj.execute("SQL")', 'change_column(:users,:name,:text)', 'change_column_default(:users,:name,from:nil,to:false)', 'change_column_default(:users,:name,{"from"=>nil,"to"=>false})', 'remove_column(:users,:name)', 'remove_column(:users,:name,:text)', 'remove_foreign_key(:users,to_table: :accounts)', 'remove_foreign_key(:users,column: :account_id)', 'remove_index(:users,column: :name)', 'remove_index(:users,name: :index_name)', 'remove_columns(:users,:name,type: :text)', 'remove_columns(:users,:name)', 'change_table(:users) { |t| t.remove :name,type: :text }', 'change_table(:users) { |t| t.remove :name }'):
        for version in (5.0,5.2,6.0,6.1,8.1):
            add('Rails/ReversibleMigration',migration % operation,TargetRailsVersion=version)
    for body in ('t.change :name,:text','(t.change :name,:text)','begin; t.change :name,:text; end','if flag; t.change :name,:text; end','flag ? t.change(:name,:text) : work','result=t.change(:name,:text)','[t.change(:name,:text)]','work { t.change(:name,:text) }','t.change(:name,:text) { work }','marker; if flag; t.change :name,:text; end','t.string :name','t.remove :name,type: :text'):
        add('Rails/ReversibleMigration',migration % ('change_table(:users) { |t| '+body+' }'))
    for operation in ('reversible { execute("SQL") }','reversible { _1; execute("SQL") }','up_only { execute("SQL") }','up_only { _1; execute("SQL") }','object.reversible { execute("SQL") }','object&.up_only { execute("SQL") }','def nested; execute("SQL"); end','drop_table(:users) { |t| t.string :name }','drop_table(:users) { _1 }','drop_table :users,&block','drop_table','table=:users; change_table(table) { |table| table.string :name }'):
        add('Rails/ReversibleMigration',migration % operation)
    for superclass in ('ActiveRecord::Migration','ActiveRecord::Migration[7]','ActiveRecord::Migration["7.0"]','::ActiveRecord::Migration[7.0]','ActiveRecord::Migration[+7.0]','ActiveRecord::Migration[-7.0]','Other::ActiveRecord::Migration[7.0]'):
        add('Rails/ReversibleMigration',f'class Example < {superclass}; def change; execute("SQL"); end; end')
    for source in ('execute("SQL")','class Example < ActiveRecord::Migration[7.0]; def up; execute("SQL"); end; end','class Example < ActiveRecord::Migration[7.0]; def self.change; execute("SQL"); end; end','module Namespace; class Example < ActiveRecord::Migration[7.0]; def change; execute("SQL"); end; end; end'):
        add('Rails/ReversibleMigration',source)
    add('Rails/ReversibleMigration',migration % 'execute("SQL")',AutoCorrect='disabled',Severity='warning')

    for database in ('mysql','postgresql','sqlite',None):
        for version in (5.0,5.2,6.0,6.1,8.1):
            for method in ('add_column','rename_column','change_column_default','change_column_null'):
                add('Rails/BulkChangeTable',f'def change; {method} :users,:a,:string; {method} :users,:b,:string; end',Database=database,TargetRailsVersion=version)
    for source in ('def change; add_column(:users,:a,:string); marker; add_column(:users,:b,:string); end','def change; add_column(:users,:a,:string); add_column(:other,:b,:string); end','def change; add_column(:users,:a,:string); add_column("users",:b,:string); end','def change; object.add_column(:users,:a,:string); object.add_column(:users,:b,:string); end','def change; add_column(table,:a,:string); add_column(table,:b,:string); end','def change; change_table(:users) { |t| t.remove :a,:b }; end','def change; change_table(:users,bulk:false) { |t| t.string :a; t.string :b }; end','def change; change_table(:users,"bulk"=>false) { |t| t.string :a; t.string :b }; end','def change; change_table(:users) { _1.string :a; _1.string :b }; end','def change; change_table(:users) { |t| (t.string :a; t.string :b) }; end','def change; change_table(:users) { |t| if flag; t.string :a; else; t.string :b; end }; end'):
        add('Rails/BulkChangeTable',source,Database='mysql')
    for table in (':users','"users"','1','1.0','1r','true','nil','1i','"users#{suffix}"','`users`'):
        add('Rails/BulkChangeTable',f'def change; add_column({table},:a,:string); add_column({table},:b,:string); end',Database='mysql')
    schema = 'ActiveRecord::Schema.define do; create_table "users" do |t| t.string "account"; t.string "tenant"; end; end'
    for validation in ('validates :account, uniqueness:true','validates :account, uniqueness:false','validates :account, uniqueness:nil','validates :missing, uniqueness:true','validates account, uniqueness:true','validates :account, uniqueness:{scope: :tenant}','validates :account, uniqueness:{scope: [:tenant,:tenant]}','validates :account, uniqueness:{scope: ["tenant",1]}','validates :account, uniqueness:{scope: %i[tenant]}','validates :account, uniqueness:{scope: %w[tenant]}','validates :account, uniqueness:{scope: :tenant.freeze}','validates :account, uniqueness:true, if:nil','validates :account, uniqueness:{conditions:nil}','validates :account, "uniqueness"=>true','self.validates :account, uniqueness:true'):
        add('Rails/UniqueValidationWithoutIndex',f'class User; {validation}; end',SchemaSource=schema)
    for index in ('t.index ["account"], unique:true','t.index ["account"], unique:false','t.index ["account"], unique:nil','t.index ["account"]','t.index [:account],unique:true','t.index "lower(account)",unique:true','t.index ["account","tenant"],unique:true'):
        indexed=schema.replace('; end; end','; '+index+'; end; end')
        add('Rails/UniqueValidationWithoutIndex','class User; validates :account, uniqueness:true; end',SchemaSource=indexed)
    add('Rails/UniqueValidationWithoutIndex','class User; validates :account, uniqueness:true; end',SchemaSource=schema,AutoCorrect='disabled',Severity='warning')

    for scope in (':tenant','[:tenant,:tenant]','["tenant",1]','%i[tenant]','%w[tenant]',':tenant.freeze','unknown'):
        for columns in ('["account"]','["account","tenant"]'):
            indexed=schema.replace('; end; end','; t.index '+columns+',unique:true; end; end')
            add('Rails/UniqueValidationWithoutIndex',f'class User; validates :account, uniqueness:{{scope: {scope}}}; end',SchemaSource=indexed)

    for table in ('`users`','`users#{suffix}`','`#{suffix}`','`#{1}`','`#{work(1)}`','%x(users)','`users\nrows`','``'):
        add('Rails/BulkChangeTable',f'def change; add_column({table},:a,:string); add_column({table},:b,:string); end',Database='mysql')
