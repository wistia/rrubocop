"""Actual production path controls for transferred schema rules."""
def rows(model):
    return [
        ('Rails/NotNullColumn','add_column :users,:name,:string,null:false','db/migrate/20990101000000_schema_parity.rb',model),
        ('Rails/NotNullColumn','add_reference :users,:name,null:false','db/schema_parity_operations.rb',model),
    ]
