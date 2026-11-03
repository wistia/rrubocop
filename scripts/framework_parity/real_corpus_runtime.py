"""Trusted dynamic keyword evaluation and numbered Array predicates."""
def programs():
    setup="$events=[]\nmodule FactoryBot\ndef self.create(name, **options); $events << [:create,name,options]; Object.new; end\ndef self.create_list(name,n,**options); Array.new(n) { create(name,**options) }; end\nend\ndef account; $events << :account; $events.count(:account); end\n"
    result=[]
    for header in ('Array.new(2)', '2.times'):
        for prefix in ('','account=9\n'):
            result.append(('FactoryBot/CreateList',setup+prefix+header+' { FactoryBot.create(:project, account:) }\np $events\n'))
    result.append(('FactoryBot/CreateList',setup+'other=Object.new\ndef other.value; $events << :value; $events.count(:value); end\nArray.new(2) { FactoryBot.create(:project, account: other&.value) }\np $events\n'))
    for selector in ('select','filter','find_all','reject'):
        result.append(('Performance/Count',f'p [1,2,3].{selector} {{ _1.odd? }}.count\n'))
    return result
