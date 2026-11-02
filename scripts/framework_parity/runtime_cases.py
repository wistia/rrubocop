"""Trusted, explicitly authored programs; ordinary fixture code is never executed."""
PROGRAMS = [
    ("Performance/Count", "items = [1, 2, 3]\np items.select { |item| item.odd? }.count\n"),
    ("Performance/Count", "items = [1, 2, 3]\np items.reject { |item| item.odd? }.count\n"),
    ("Performance/Count", "items = [1, 2, 3]\npredicate = ->(item) { item.odd? }\np items.reject(&predicate).count\n"),
    ("Performance/Size", "p [1, 2, 3].count\n"),
    ("Performance/RangeInclude", "p (1..10).include?(5)\n"),
    ("Performance/UriDefaultParser", "require 'uri'\np URI::Parser.new.escape('a b')\n"),
    ("Rails/Blank", "require 'active_support/core_ext/object/blank'\n[nil, [], [1], '', 'text'].each { |value| p(value.nil? || value.empty?) }\n"),
    ("Rails/Present", "require 'active_support/core_ext/object/blank'\n[nil, [], [1], '', 'text'].each { |value| p(value && !value.empty?) }\n"),
    ("FactoryBot/FactoryClassName", "class User; end\ndef factory(name, **options); p options[:class].to_s; end\nfactory :user, class: User\n"),
    ("FactoryBot/CreateList", "$calls = []\ndef create(name, **options); $calls << [name, options]; Object.new; end\ndef create_list(name, count, **options); Array.new(count) { create(name, **options) }; end\n3.times { create :user, age: 18 }\np $calls\n"),
]
