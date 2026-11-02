"""Pinned original-cop branch specimens for the framework parity queue."""

def cases():
    rows = []
    def add(cop, source, **config):
        rows.append(dict(id=f"{cop}:{len(rows)}", cop=cop, path="app/models/example.rb", source=source + "\n", expected=[], corrected=source + "\n", config=config))
    for selector in ("select", "filter", "find_all", "reject"):
        for counter in ("count", "length", "size"):
            add("Performance/Count", f"items.{selector} {{ |item| item.active? }}.{counter}")
        add("Performance/Count", f"items.{selector}(&:active?).count")
        add("Performance/Count", f"items&.{selector} {{ |item| item.active? }}&.count")
    for source in ("items.select.count", "items.select(:name).count", "items.select { |item| item.active? }.count(1)", "items.select { |item| item.active? }.count { |item| item.ready? }", "items.reject { |item| log(item); item.active? }.count", "items.reject(&predicate).count", "items.reject { |item| # keep this\n item.active? }.count"):
        add("Performance/Count", source)
    for source in ("items.nil? || items.empty?", "items == nil || items.empty?", "nil == items || items.empty?", "!items || items.empty?", "items.nil? || !!items.empty?", "nil? || empty?", "!items.present?", "!present?", "not items.present?", "def blank?; if ready?; !present?; end; end", "items.nil? || other.empty?", "items.nil? && items.empty?", "items&.nil? || items&.empty?", "def blank?; !present?; end", "perform unless items.present?", "unless items.present?\n perform\nend", "unless items.present?\n perform\nelse\n other\nend"):
        add("Rails/Blank", source)
    for option, source in (("NilOrEmpty", "items.nil? || items.empty?"), ("NotPresent", "!items.present?"), ("UnlessPresent", "perform unless items.present?")):
        add("Rails/Blank", source, **{option:False})
    for source in ("[1, 2].count", "{a: 1}.count", "items.to_a.count", "items.to_a { |n| n }.count", "items&.to_h&.count", "Array[1].count", "Hash[:a, 1].count", "Array(items).count", "Hash(items).count", "items.count", "[1, 2].count(1)", "[1].count { |n| n.odd? }"):
        add("Performance/Size", source)
    for source in ("URI::Parser.new", "::URI::Parser.new", "URI::Parser.new(options)", "Other::URI::Parser.new", "URI::Parser&.new", "URI::Parser.new { foo }"):
        add("Performance/UriDefaultParser", source)
    for source in ("(1..10).include?(n)", "(1...10).member?(n)", "(1..10)&.include?(n)", "((1..10)).include?(n)", "range.include?(n)", "('a'..'z').include?(text)"):
        add("Performance/RangeInclude", source)
    for source in ("!items.blank?", "!blank?", "items && !items.empty?", "!items.nil? && !items.empty?", "!!items && !items.empty?", "items != nil && !items.empty?", "items && !other.empty?", "perform unless items.blank?", "unless items.blank?\n perform\nend", "items&.blank?", "!items&.blank?"):
        add("Rails/Present", source)
    for option, source in (("NotBlank", "!items.blank?"), ("NotNilAndNotEmpty", "items && !items.empty?"), ("UnlessBlank", "perform unless items.blank?")):
        add("Rails/Present", source, **{option: False})
    for source in ("input.html_safe", "input&.html_safe", "raw(input)", "safe_concat(input)", "output.safe_concat(input)", "object.raw(input)", "\"safe\".html_safe", "\"#{input}\".html_safe", "I18n.t(:name).html_safe", "raw(I18n.t(:name))", "raw(t(:name))", "I18n.translate(:x).html_safe", "object.t(:name).html_safe", "html_safe", "input.html_safe(extra)"):
        add("Rails/OutputSafety", source)
    for cop, source in (("Rails/Blank", "items.nil? || items.empty?"), ("Performance/Count", "items.select { |item| item.active? }.count"), ("Performance/Size", "[1].count"), ("Performance/UriDefaultParser", "URI::Parser.new"), ("Performance/RangeInclude", "(1..3).include?(n)"), ("Rails/Present", "!items.blank?")):
        add(cop, source, AutoCorrect="disabled")
    add("Performance/Count", "éléments.select { |élément| élément.active? }.count")
    for cop, source in (("Rails/Blank", "élèves.nil? || élèves.empty?"), ("Rails/Blank", "items.nil? ||\n items.empty?"), ("Rails/Present", "élèves && !élèves.empty?"), ("Rails/Present", "items &&\n !items.empty?")):
        add(cop, source)
    for source in ("factory :foo, class: Foo", "factory(:foo, class: Foo)", "factory :foo, class: Admin::Foo", "factory :foo, class: ::Foo", "factory :foo, class: Hash", "factory :foo, class: ::Hash", "factory :foo, class: OpenStruct", "factory :foo, class: Admin::Hash", "factory :foo, class: 'Foo'", "factory :foo, class: dynamic_class", "factory :foo, other: true, class: Foo", "builder.factory :foo, class: Foo", "factory :foo, { class: Foo }", "factory :foo, { :class => Foo }", "factory :foo, { 'class' => Foo }"):
        add("FactoryBot/FactoryClassName", source)
    add("FactoryBot/FactoryClassName", "factory :foo, class: Foo", AutoCorrect="disabled")
    for source in ("factory :user do\n name \"Ada\"\nend", "factory :user do\n name(\"Ada\")\nend", "factory :user do |f|\n f.name \"Ada\"\nend", "factory :user do\n self.name \"Ada\"\nend", "factory :user do\n other.name \"Ada\"\nend", "factory :user do\n name { \"Ada\" }\nend", "factory :user do\n account factory: :account\nend", "factory :user do\n options key: :value\nend", "factory :user do\n options(key: :value)\nend", "factory :user do\n name(&generator)\nend", "name \"Ada\"", "trait :admin do\n admin true\nend", "transient do\n count 3\nend", "factory :user do\n sequence(:email) { |n| n }\nend", "factory :user do\n name (1)\nend"):
        add("FactoryBot/AttributeDefinedStatically", source)
    add("FactoryBot/AttributeDefinedStatically", "factory :user do\n name \"Ada\"\nend", AutoCorrect="disabled")
    for source in ("3.times { create :user }", "3.times.map { create(:user) }", "Array.new(3) { create :user }", "3.times { FactoryBot.create :user }", "3.times { ::FactoryGirl.create :user }", "1.times { create :user }", "3.times { |n| create :user, age: n }", "3.times { create :user, age: rand }", "3.times { create :user, age: random_age() }", "3.times { other.create :user }", "[create(:user), create(:user)]", "[create(:user, age: rand), create(:user, age: rand)]", "[create(:user), create(:admin)]", "0x3.times { create :user }", "3.times { create :user, age: 18 }", "3.times { create(:user, age: 18, active: true) }"):
        add("FactoryBot/CreateList", source)
    for source in ("create_list :user, 3", "create_list(:user, 3)", "FactoryBot.create_list(:user, 3, age: 18)", "create_list :user, 1", "[create(:user), create(:user)]", "create_list(:user, 3) { |user| user.active = true }"):
        add("FactoryBot/CreateList", source, EnforcedStyle="n_times")
    add("FactoryBot/CreateList", "3.times { create :user }", ExplicitOnly=True)
    add("FactoryBot/CreateList", "3.times { FactoryBot.create :user }", ExplicitOnly=True)
    add("FactoryBot/CreateList", "3.times { create :user }", AutoCorrect="disabled")
    for source in ("3.times { create(:user) { |user| user.active = true } }", "3.times { create(:user) { notify } }", "3.times do\n create(:user) do |user|\n  user.active = true\n  user.name = 'Ada'\n end\nend", "age = 18\n3.times { create :user, age: age }", "3.times { |unused| create :user }", "create_list(:user, 0x3)"):
        add("FactoryBot/CreateList", source, **({"EnforcedStyle":"n_times"} if source.startswith("create_list") else {}))
    from runtime_cases import PROGRAMS
    for cop, source in PROGRAMS:
        rows.append(dict(id=f"{cop}:runtime:{len(rows)}", cop=cop, path="app/models/example.rb", source=source, expected=[], corrected=source, config={}, trusted_runtime=True))
    for source in ("[1, 2].size", "{}.length", "'text'.size", ":text.size", "'text'.count('t')", "'text'.count(variable)", "[1, 2].count(1)", "COUNT = [1, 2].size", "items.each { [1, 2].size }", "[*items].size", "{ **options }.size", "[1].count { |n| n.odd? }", "\"#{value}\".size", "items.size", "[1]&.size"):
        add("Performance/FixedSize", source)
    for source in ("items.map { |n| [n] }.flatten(1)", "items.collect { |n| [n] }.flatten(1)", "items.map(&:children).flatten(1)", "items&.map { |n| [n] }&.flatten(1)", "items.map { |n| [n] }.flatten", "items.map { |n| [n] }.flatten(2)", "items.map { |n| [n] }.flatten(depth)", "items.map.flatten(1)"):
        add("Performance/FlatMap", source)
    add("Performance/FlatMap", "items.map { |n| [n] }.flatten", EnabledForFlattenWithoutParams=True)
    add("Performance/FlatMap", "items.map { |n| [n] }.flatten(1)", AutoCorrect="disabled")
    for source in ("Table[\"*\"]", "Table['*']", "User.arel_table[\"*\"]", "arel_table[\"*\"]", "table[\"*\"]", "Table[:*]", "Table[\"name\"]", "Table[\"*\", :name]", "Table.[](\"*\")", "Table[%q{*}]", "Table[\"\\x2A\"]"):
        add("Rails/ArelStar",source)
    for source in ("User.where(id: 1).take", "User.where(id: 1).first", "User.where(id: 1).take(2)", "User.where(id: 1)&.take", "User&.where(id: 1)&.take", "where(id: 1).take", "User.where(id: 1).take()", "User.where { |x| x.active? }.take"):
        add("Rails/FindBy",source)
    add("Rails/FindBy", "User.where(id: 1).first", IgnoreWhereFirst=False)
    for source in ("User.pluck(:name).first", "User.pluck(:name, :age).first", "User.pluck(:name).first(2)", "User&.pluck(:name)&.first", "pluck(:name).first", "User.pluck(:name).first()", "User.pluck.first"):
        add("Rails/Pick",source)
    add("Rails/ArelStar", "arel_table = table\narel_table[\"*\"]")
    for cop in ("Performance/FixedSize", "Performance/Size"):
        for source in ("%w[a b].count", "%i[a b].count"):
            add(cop, source)
    for source in ("items.map { |n| [n] }.flatten(0x1)", "items.map { |n| [n] }.flatten(+1)"):
        add("Performance/FlatMap",source)
    return rows
