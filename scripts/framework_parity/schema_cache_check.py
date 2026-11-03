#!/usr/bin/env python3
"""Compare real CLI cache behavior across external schema/database mutations."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--ruby-project',type=Path,required=True)
p.add_argument('--native',type=Path,required=True)
p.add_argument('--output',type=Path,required=True)
a=p.parse_args()
a.output.mkdir(parents=True,exist_ok=True)
results=[]
with tempfile.TemporaryDirectory(prefix='rrubocop-schema-cache-',dir='/dev/shm') as directory:
    root=Path(directory)
    (root/'db').mkdir();(root/'app/models').mkdir(parents=True);(root/'config').mkdir()
    (root/'db/migrate').mkdir()
    config=root/'.rubocop.yml'
    config.write_text('require: [rubocop-rails]\nAllCops:\n  TargetRubyVersion: 3.4\n  TargetRailsVersion: 8.1\n  DisabledByDefault: true\nRails/UniqueValidationWithoutIndex:\n  Enabled: true\nRails/NotNullColumn:\n  Enabled: true\n')
    model=root/'app/models/cache_schema_row.rb'
    model.write_text('class CacheSchemaRow; validates :token, uniqueness:true; end\n')
    migration=root/'db/migrate/20990101000000_cache_schema.rb'
    migration.write_text('add_column :cache_schema_rows,:token,:text,null:false\n')
    env={**os.environ,'ASDF_RUBY_VERSION':'3.4.9','BUNDLE_GEMFILE':str(a.ruby_project/'Gemfile'),'RRUBOCOP_CACHE_DIR':str(root/'native-cache')}
    env.pop('DATABASE_URL',None)
    def observe(command,cop,file,cache):
        run=subprocess.run(command+['--config',str(config),'--only',cop,'--format','json','--cache',str(cache).lower(),str(file)],cwd=root,env=env,text=True,capture_output=True,timeout=30)
        if run.returncode not in (0,1):raise RuntimeError(run.stderr)
        value=json.loads(run.stdout)
        offenses=value.get('offenses')
        if offenses is None:offenses=[o for f in value.get('files',[]) for o in f['offenses']]
        return dict(count=sum(o['cop_name']==cop for o in offenses),inspected=value.get('metadata',{}).get('files_inspected',value.get('summary',{}).get('inspected_file_count')))
    def check(label,cop,file,expected):
        ruby=observe([str(a.ruby_project/'bin/rubocop-ruby')],cop,file,False)
        native=observe([str(a.native)],cop,file,True)
        results.append(dict(label=label,cop=cop,ruby=ruby,native=native,expected=expected,matches=ruby==native and ruby['count']==expected and ruby['inspected']==1))
    schema=root/'db/schema.rb'
    base='ActiveRecord::Schema.define do; create_table "cache_schema_rows" do |t| t.string "token"; t.string "other"; %s; end; end\n'
    check('schema absent','Rails/UniqueValidationWithoutIndex',model,0)
    schema.write_text(base % '')
    check('schema created without index','Rails/UniqueValidationWithoutIndex',model,1)
    check('unchanged cached model','Rails/UniqueValidationWithoutIndex',model,1)
    schema.write_text(base % 't.index ["token"],unique:true')
    check('unique index added','Rails/UniqueValidationWithoutIndex',model,0)
    stat=schema.stat()
    schema.write_text(base % 't.index ["other"],unique:true')
    os.utime(schema,ns=(stat.st_atime_ns,stat.st_mtime_ns))
    assert schema.stat().st_size==stat.st_size
    check('same size and mtime index edit','Rails/UniqueValidationWithoutIndex',model,1)
    schema.unlink()
    check('schema deleted','Rails/UniqueValidationWithoutIndex',model,0)
    database=root/'config/database.yml'
    database.write_text('development:\n  adapter: mysql2\n')
    check('mysql adapter','Rails/NotNullColumn',migration,0)
    database.write_text('development:\n  adapter: postgresql\n')
    check('yaml adapter changes','Rails/NotNullColumn',migration,1)
    database.unlink();env['DATABASE_URL']='mysql2://localhost/example'
    check('environment mysql adapter','Rails/NotNullColumn',migration,0)
    env['DATABASE_URL']='postgres://localhost/example'
    check('environment postgres adapter','Rails/NotNullColumn',migration,1)
    env.pop('DATABASE_URL',None)
    config.write_text(config.read_text().replace('  TargetRailsVersion: 8.1\n','')+'Rails/BulkChangeTable:\n  Enabled: true\n  Database: postgresql\n')
    migration.write_text('def change; add_column :cache_schema_rows,:name,:string; remove_column :cache_schema_rows,:other; end\n')
    lockfile=root/'Gemfile.lock'
    for version,expected in ((None,0),('5.1.0',0),('5.2.0',1),('6.0.0',1),('6.1.0',1)):
        if version is None:
            if lockfile.exists():lockfile.unlink()
        else:
            lockfile.write_text('GEM\n  remote: https://rubygems.org/\n  specs:\n    railties ('+version+')\n\nPLATFORMS\n  ruby\n\nDEPENDENCIES\n  railties\n')
        check('lockfile railties '+str(version),'Rails/BulkChangeTable',migration,expected)
    assert (root/'native-cache/cache.redb').exists(),'native result cache was not created'
(a.output/'results.json').write_text(json.dumps(results,indent=2)+'\n')
summary=dict(controls=len(results),differences=sum(not row['matches'] for row in results))
(a.output/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps(summary,indent=2))
raise SystemExit(bool(summary['differences']))
