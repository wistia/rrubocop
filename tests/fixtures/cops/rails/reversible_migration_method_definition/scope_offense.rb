class X < ActiveRecord::
 Migration[7.0]; end
# rrubocop-expect: 1:0 Rails/ReversibleMigrationMethodDefinition: Migrations must contain either a `change` method, or both an `up` and a `down` method.
