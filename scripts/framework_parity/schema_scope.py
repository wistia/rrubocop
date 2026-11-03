"""Actual production path controls for transferred schema rules."""
def rows(model):
    return [
        ('Rails/ReversibleMigration','class ExampleMigration < ActiveRecord::Migration[7.0]; def change; execute("SQL"); end; end','db/migrate/20990101000000_schema_parity.rb',model),
        ('Rails/UniqueValidationWithoutIndex','class Account; validates :name, uniqueness:true; end',model,'app/services/schema_parity.rb'),
        ('Rails/NotNullColumn','add_column :users,:name,:string,null:false','db/migrate/20990101000000_schema_parity.rb',model),
        ('Rails/NotNullColumn','add_reference :users,:name,null:false','db/schema_parity_operations.rb',model),
    ]

def extra_cases(model):
    source='def change; add_column :users,:name,:string; remove_column :users,:other; end\n'
    return [dict(cop='Rails/BulkChangeTable',source=source,path=path,positive=positive,schema_database='mysql')
            for path,positive in [('db/migrate/20990101000000_schema_parity.rb',True),(model,False)]]
