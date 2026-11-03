"""Only authored observation programs are executed; imported examples are never run."""
def programs():
    rows=[]
    prelude="require 'pathname'\nmodule Rails; def self.root; Pathname.new('/tmp/project'); end; end\n"
    for expression in ("Rails.root.join('app','models')", "Rails.root.join('app/models')", "File.join(Rails.root,'app','models')", '"#{Rails.root}/app/models"', '"#{Rails.root.join(\'app\')}.rb"', "Rails.root.join('app','/models')", "Rails.root.join('/app/models')"):
        for style in ('slashes','arguments'):
            rows.append(('Rails/FilePath', prelude+'p ('+expression+').to_s\n', {'EnforcedStyle':style}))
    for expression in ("File.join(Rails.root,['app','models'])", "File.join Rails.root, 'app','models'"):
        rows.append(('Rails/FilePath', prelude+'result='+expression+'\np [result.to_s,result.class.name]\n', {}))
    for expression in ('"#{Rails.root}/a/../b"', '"#{Rails.root}/a//b"', '"#{Rails.root}/a/#{name}"'):
        rows.append(('Rails/FilePath',prelude+'name="../b"\np ('+expression+').to_s\n',{}))
    prelude="require 'active_support/core_ext/module/delegation'\nclass Target\nattr_reader :events\ndef initialize; @events=[]; end\ndef foo(value=:default); @events << value; [:foo,value]; end\nend\nclass Probe\nattr_reader :target\ndef initialize; @target=Target.new; end\n"
    for method,call in (("def foo(value); target.foo(value); end",'foo(7)'),('def target_foo(value); target.foo(value); end','target_foo(7)'),('private; def foo(value); target.foo(value); end; public','send(:foo,7)')):
        source=prelude+method+"\nend\nprobe=Probe.new\np probe."+call+"\np probe.target.events\n"
        rows.append(('Rails/Delegate',source,{}))
    prelude="$events=[]\ndef value; $events << :value; 7; end\ndef options; $events << :options; {id: value}; end\ndef session; $events << :session; {token: 2}; end\ndef get(action,*args,**kwargs)\nparams=kwargs.key?(:params) ? kwargs[:params] : args[0]\nsession=kwargs.key?(:session) ? kwargs[:session] : args[1]\np [action,params || {},session || {}]\nend\n"
    for expression in ('get :show, {id: value}', 'get(:show, options, session)', 'get :show, {}, {}', 'get :show, {id: value}, {token: 2}'):
        rows.append(('Rails/HttpPositionalArguments',prelude+expression+'\np $events\n',{'TargetRailsVersion':5.0}))
    return rows
