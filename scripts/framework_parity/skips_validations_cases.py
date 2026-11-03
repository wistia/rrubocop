"""Original configuration precedence, ordinary/safe sends and typed exemption keys."""
def extend(add):
    import json
    from pathlib import Path
    original_add=add
    def add(cop, source, **config):
        config.setdefault('AllowedMethods',[])
        original_add(cop,source,**config)
    cop='Rails/SkipsModelValidations' 
    for row in json.loads((Path(__file__).parents[2]/'tests/fixtures/framework-skips-original-spec.json').read_text())['cases']:
        add(cop,row['source'])
    methods='decrement! decrement_counter increment! increment_counter insert insert! insert_all insert_all! toggle! touch touch_all update_all update_attribute update_column update_columns update_counters upsert upsert_all'.split()
    for name in methods:
        for prefix in ('', 'User.', 'User&.'):
            for args in ('', '()', '(0)', '(true)', '(false)', '(a:1,b:2)', '(&block)'):
                add(cop,prefix+name+args)
    for prefix in ('FileUtils.', '::FileUtils.', 'Other::FileUtils.', 'FileUtils&.', 'object&.', 'object.'):
        for arg in ('true','false','(true)','nil','*values','**options'):
            add(cop,prefix+'touch('+arg+')')
    for method in ('insert','insert!'):
        for prefix in ('', 'User.', 'User&.'):
            for second in ('{}','{returning: false}','{unique_by: :key}','{other: true}','{"other" => true}','{"other": true}', '{**options}', '{returning: false, **options}', '{returning: false, other: true}', 'returning: false','other: true','**options','nil','value'):
                add(cop,f'{prefix}{method}(attributes, {second})')
    for config in ({'AllowedMethods':['touch']},{'ForbiddenMethods':['touch']},{'ForbiddenMethods':[]},{'Blacklist':['touch']},{'Whitelist':['touch']},{'Blacklist':None},{'Whitelist':None},{'Blacklist':False},{'Whitelist':False},{'ForbiddenMethods':None}):
        for source in ('touch','object.touch','object&.touch','object.toggle!(:attr)','object.update_all(flag: true)'):
            add(cop,source,**config)
    for source in ('touch=1; touch','def touch; end','def self.touch; end','alias touch old','undef touch','touch # rubocop:disable Rails/SkipsModelValidations','obj.touch = 1','obj.touch=1','obj&.touch = 1','value + 1','obj[0]'):
        add(cop,source,ForbiddenMethods=['touch','touch=','+','[]'])

    for source in ('touch','touch_all','User.touch(:attr)','User.update_all(flag: true)'):
        add(cop,source,AllowedMethods=['touch','touch_all'])
    for source, method in [('!value','!'),('not value','!'),('obj[0]','[]'),('obj[0]=1','[]='),('obj.touch = 1','touch=')]:
        add(cop,source,ForbiddenMethods=[method])

    for source in ('{touch:}', '{touch_all:}', 'touch=1; {touch:}', '{Touch:}', '{_touch:}', 'def f(touch); {touch:}; end'):
        add(cop,source)
