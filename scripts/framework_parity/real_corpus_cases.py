"""Minimized current Wistia mismatches, preserving typed Ruby role controls."""
def extend(add):
    cop='FactoryBot/CreateList'
    for count in ('Array.new(2)', '2.times', '2.times.map'):
        for call in ('create(:project, account:)', 'FactoryBot.create(:project, account:)', 'create(:project, account: account)', 'create(:project, account: other&.value)', 'create(:project, account: other + 1)', 'create(:project, account: __LINE__)', 'create(:project, account: __FILE__)', 'create(:project, account: __ENCODING__)'):
            for setup in ('','account=1; '):
                add(cop,setup+count+' { '+call+' }')
    add(cop,'2.times do\n create(:video_remix, account:, contact:, remix_conversation: conversation, status: VideoRemix::STATUS_EDIT_TREE_GENERATED, created_at: stamped_at)\nend')
    for setup in ('','account=1; '):
        add(cop,setup+'[create(:project, account:), create(:project, account:)]')
    for ruby in (2.5,2.7,3.4):
        for selector in ('select','filter','find_all','reject'):
            for counter in ('count','length','size'):
                for body in ('_1.active?', '|value| value.active?', 'items.any? { _1.active? }', '|value| value._1', '"_1"'):
                    add('Performance/Count',f'items.{selector} {{ {body} }}.{counter}',TargetRubyVersion=ruby)
    add('Performance/Count','participants.select { _1.kind == :STANDARD && !_1.attributes["role"].in?(["recorder", "audience"]) }.count')

    for ruby in (2.5,2.7,3.4):
        for source in ('2.times { create(:project, account: _1) }','Array.new(2) { create(:project, account: _1) }','2.times.map { create(:project, account: _1) }'):
            add('FactoryBot/CreateList',source,TargetRubyVersion=ruby)
    add('FactoryBot/CreateList','other=Object.new; Array.new(2) { create(:project, account: other&.value) }')
