"""Original relation/send ancestry, allowed method traversal, and unsafe result roles."""
def extend(add):
    import json
    from pathlib import Path
    cop='Rails/FindEach'
    for row in json.loads((Path(__file__).parents[2]/'tests/fixtures/framework-find-each-original-spec.json').read_text())['cases']:
        add(cop,row['source'])
    for scope in ('all','where','includes','not','order','limit'):
        for receiver in ('User.', 'User&.', '', 'errors.', 'model.errors.', 'model&.errors.'):
            for suffix in ('.each', '&.each', '.each { |row| row.name }', '.each { _1.name }', '.each(&:name)', '.each(1)'):
                for parent in ('', 'class C < ApplicationRecord; %s; end'):
                    body=receiver+scope+suffix
                    add(cop,parent%body if parent else body)
    for parent in ('ApplicationRecord','::ApplicationRecord',':: ApplicationRecord','Other::ApplicationRecord','ActiveRecord::Base','::ActiveRecord::Base','Other::ActiveRecord::Base','(ApplicationRecord)'):
        for body in ('all.each','all=items; all.each','where.each','errors.where(:name).each','errors=items; errors.where(:name).each','all { order }.each','class Inner; all.each; end'):
            add(cop,f'class C < {parent}; {body}; end')
    for config in ({'AllowedMethods':[]},{'AllowedMethods':['order']},{'AllowedMethods':'order'},{'IgnoredMethods':['order']},{'ExcludedMethods':['order']},{'AllowedPatterns':['ord']},{'IgnoredPatterns':['ord']},{'AllowedMethods':None},{'AllowedPatterns':None}):
        for source in ('User.order(:name).all.each','User.where(value: order).each','User.where(value: order()).each','User.where(value: other&.order).each','User.where(value: work { order }).each','User.all.each { order }','order=1; User.where(value: order).each'):
            add(cop,source,**config)
    for source in ('User.all.each # rubocop:disable Rails/FindEach','User.all.each = value','User.all.each','result=User.all.each { |v| v }','puts(User.all.each { |v| v })','User.all.each { |v| v }; puts :done'):
        add(cop,source)
    add(cop,'User.all.each',AutoCorrect='disabled')
