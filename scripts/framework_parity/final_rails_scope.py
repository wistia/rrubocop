"""Actual production path policies for the final three Rails cops."""
ROWS=[
 ('Rails/Delegate','class Model; def foo; target.foo; end; end','app/models/framework_final_probe.rb','app/controllers/framework_final_probe.rb'),
 ('Rails/FilePath',"Rails.root.join('app','models')",'app/models/framework_final_probe.rb','app/services/framework_final_probe.rb'),
 ('Rails/HttpPositionalArguments','get :show, id: 1','spec/requests/framework_final_probe_spec.rb','app/models/framework_final_probe.rb'),
]
def extra_cases():
    return [dict(cop='Rails/HttpPositionalArguments',source='get :show, id: 1\n',path=path,positive=positive)
            for path,positive in (('test/controllers/framework_final_probe_test.rb',True),('app/models/framework_final_probe_spec.rb',False))] + [dict(cop=cop,source=source+' # rubocop:disable '+cop+'\n',path=path,positive=False) for cop,source,path,_ in ROWS]
