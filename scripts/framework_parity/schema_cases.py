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
