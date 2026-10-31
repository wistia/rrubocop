add_foreign_key :widgets, :accounts
# rrubocop-expect: 1:0 Cops/NoForeignKeys: Foreign keys are ignored via database config. Use a different way to enforce referential integrity.
