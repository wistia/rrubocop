"""Original migration class matcher and descendant method/argument roles."""
def extend(add):
    import json
    from pathlib import Path
    cop='Rails/ReversibleMigrationMethodDefinition'
    for row in json.loads((Path(__file__).parents[2]/'tests/fixtures/framework-reversible-method-original-spec.json').read_text())['cases']:
        add(cop,row['source'])
    for name in ('Example','::Example','Other::Example'):
        for parent in ('ActiveRecord::Migration[6.0]','::ActiveRecord::Migration[6.0]','Other::ActiveRecord::Migration[6.0]','ActiveRecord::Migration[6]','ActiveRecord::Migration[+ 6.0]','ActiveRecord::Migration[-6.0]','ActiveRecord::Migration[6.0,7.0]','ActiveRecord::Migration.[](6.0)','ActiveRecord::Migration&.[](6.0)','(ActiveRecord::Migration[6.0])','Migration[6.0]'):
            for body in ('','def change; end','def up; end','def up; end; def down; end','def change(a); end','class Inner; def change; end; end'):
                add(cop,f'class {name} < {parent}\n{body}\nend')
    for body in ('def change(); end','def change( # comment\n); end','def self.change; end','def self.up; end; def self.down; end','def change(...); end','def change(**nil); end','def change(&); end','def up(a); end; def down; end','if ready?; def change; end; end','def helper; def change; end; end','define_method(:change) {}','def change; end; class Inner < ActiveRecord::Migration[7.0]; end','def up; end; class Inner; def down; end; end','private; def change; end'):
        add(cop,f'class Example < ActiveRecord::Migration[6.0]\n{body}\nend')
