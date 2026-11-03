"""Pinned original and branch specimens for the final three Rails rules."""
import json
from pathlib import Path

def cases():
    rows=[]
    def add(cop,source,**config):
        if cop.endswith("HttpPositionalArguments"):
            config.setdefault("GemVersions", {"railties":"8.1.3.1"})
        rows.append(dict(id=f'{cop}:final:{len(rows)}',cop=cop,path='spec/requests/example_spec.rb' if cop.endswith('HttpPositionalArguments') else 'app/models/example.rb',source=source if source.endswith('\n') else source+'\n',expected=[],corrected=source,config=config))
    for row in json.loads((Path(__file__).parents[2]/'tests/fixtures/final-rails-upstream/cases.json').read_text()):
        config=dict(row['config'])
        if row['cop'].endswith('HttpPositionalArguments') and 'TargetRailsVersion' in config:
            config['GemVersions']={'railties':str(config['TargetRailsVersion'])}
        add(row['cop'],row['source'],**config)
    for receiver in ('target', 'target()', 'self', 'TARGET', '@target', 'target(1)', 'target.child'):
        for declaration,forward in (('', ''), ('(value)', '(value)'), ('(value,other)', '(value,other)'), ('(value=1)', '(value)'), ('(value:)', '(value)'), ('(*values)', '(*values)'), ('(&block)', '(&block)'), ('(...)', '(...)')):
            add('Rails/Delegate',f'class Model; def foo{declaration}; {receiver}.foo{forward}; end; end')
    for source in ('def foo; target.foo; end', 'def target_foo; target.foo; end', 'def self.foo; target.foo; end', 'def foo; (target.foo); end', 'def foo; begin; target.foo; end; end', 'def foo; target.foo {work}; end', 'def foo; target&.foo; end', 'def foo; target.foo(); end', 'def []=(value); target[value]; end', 'def foo; # keep\n target.foo; end'):
        for config in ({},{'EnforceForPrefixed':False},{'AutoCorrect':'disabled'},{'Severity':'warning'}):
            add('Rails/Delegate',source,**config)
    for before,after in (('private;', ''),('protected;', ''),('public;', ''),('', 'private :foo'),('', 'protected :foo'),('', 'private :foo; public :foo'),('private;', 'public :foo'),('private :other;', ''),('self.private;', ''),('private;', 'class Inner;public;end')):
        add('Rails/Delegate',f'class Model; {before} def foo; target.foo; end; {after}; end')
    for prefix in ('private ', 'protected ', 'public ', 'private(', 'protected(', 'public('):
        suffix=')' if prefix.endswith('(') else ''
        add('Rails/Delegate',f'class Model; {prefix}def foo; target.foo; end{suffix}; end')
    for verb in ('get','post','put','patch','delete','head','process'):
        for data in ('{id: 1}', 'id: 1', '{}', 'nil', 'false', 'options', '**options', '{**options}', '{id: 1, **options}', '{format: :json}', '{format: :json, id: 1}', '{params: {id: 1}}', '{"params" => 1}', '{to: :show}', '{id:}', '{id: 1}, {}', '{id: 1}, session', '{id: 1}, second, third'):
            add('Rails/HttpPositionalArguments',f'{verb} :show, {data}',TargetRailsVersion=5.0)
    for version in (4.2,5.0,5.1,8.1):
        for source in ('get :show, id: 1','self.get :show, id: 1','object&.get :show, id: 1', 'get nil, id: 1', 'get(:show, {id: 1}, {})', 'get :show, # keep\n id: 1', 'routes do; get :show, id: 1; end', 'draw do; get :show, id: 1; end', 'routes { _1; get :show, id: 1 }', 'include Rack::Test::Methods; get :show, id: 1', 'get :show, id: 1; include Rack::Test::Methods', 'include ::Rack::Test::Methods; get :show, id: 1', 'other.include Rack::Test::Methods; get :show, id: 1'):
            add('Rails/HttpPositionalArguments',source,TargetRailsVersion=version)
    add('Rails/HttpPositionalArguments','get :show, id: 1',AutoCorrect='disabled')
    add('Rails/HttpPositionalArguments','get :show, id: 1',Severity='error')
    for style in ('slashes','arguments'):
        for source in ("Rails.root.join('app','models')", "::Rails.root.join('app','models')", "Other::Rails.root.join('app','models')", "Rails.root(1).join('app','models')", "Rails.root&.join('app','models')", "Rails.root.join('app/models')", "Rails.root.join('/app//models/')", "Rails.root.join(%q(app/models))", "Rails.root.join('app', variable)", "File.join(Rails.root,'app','models')", "::File.join(::Rails.root,'app')", "Other::File.join(Rails.root,'app')", "File.join('app',Rails.root)", "File.join(Rails.root,'/app')", "Rails.root.join('a', # keep\n'b')", '"#{Rails.root}/app/models"', '"prefix #{Rails.root}/app/#{name}"', '\"#{Rails.root.join(\'app\')}.rb\"', '\"#{Rails.root}.rb\"', '\"#{Rails.root}:/app\"', '\"#{Rails.root}/app:other\"', '\"#{Rails.root}/app/#{Rails.root}/b\"'):
            add('Rails/FilePath',source,EnforcedStyle=style)
    add('Rails/FilePath',"Rails.root.join('app','models')",AutoCorrect='disabled')
    add('Rails/FilePath',"Rails.root.join('app','models')",Severity='warning')
    for gems in ({},{'railties':'4.2.11.3'},{'railties':'5.0.0'},{'railties':'5.1.0'},{'railties':'8.1.3.1'}):
        for version in (4.2,5.0,8.1):
            add('Rails/HttpPositionalArguments','get :show, id: 1',GemVersions=gems,TargetRailsVersion=version)
    for source in ('def foo = target.foo','def foo(value) = target.foo(value)','def foo; return target.foo; end','def foo; target.foo; rescue; fallback; end','def foo=(value); target.foo=value; end','def [](value); target[value]; end','def foo; target.foo(&block); end','def foo; target.foo; ensure; cleanup; end'):
        add('Rails/Delegate',source)
    for style in ('slashes','arguments'):
        for source in ("Rails.root.join('été','模型')", "File.join(Rails.root,'été')", "Rails.root.join('app', '/models')", "Rails.root.join(%q(app/models))", "Rails.root.join(%Q(app/models))", "Rails.root.join('app\\nmodels')", "Rails.root.join('a', %q(#{value}))", "Rails.root.join('a', 'b\"c')", "Rails.root.join('a', 'b\\c')", "Rails.root.join('a', 'b\nc')", "Rails.root.join('a', '', 'b')", "wrapper(Rails.root).join('a','b')", "Rails.root.join(:app,:models)", "Rails.root.join('a',*parts)", "Rails.root.join('a','b') {work}"):
            add('Rails/FilePath',source,EnforcedStyle=style)
    for source in ('get(:show, {id: 1}) {work}', 'get :show, {id: 1} do; work; end', "File.join(Rails.root,'a','b') {work}"):
        add('Rails/FilePath' if source.startswith('File') else 'Rails/HttpPositionalArguments',source)
    add("Rails/FilePath", "File.join Rails.root, 'app', 'models'")
    add('Rails/FilePath', '\"#{Rails.root.join(\'icon\')}.p\'ng\"')
    from final_rails_runtime import programs
    for cop,source,config in programs():
        add(cop,source,**config)
        rows[-1]['trusted_runtime']=True
    return rows
