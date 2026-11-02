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
    for source in ("caller.first", "caller_locations.first", "caller[0]", "caller(2)[3]", "caller_locations(2).first", "Kernel.caller.first", "caller(1, 2).first", "caller.first(2)", "caller = frames\ncaller.first", "caller.[](2)", "caller&.first", "caller(0x2)[0x3]"):
        add("Performance/Caller",source)
    for source in ("options.keys.include?(:name)", "options.values.include?('x')", "options&.keys&.include?(:name)", "options.keys&.include?(:name)", "keys.include?(:name)", "options.keys.include?(:a, :b)", "options.keys().include?(:name)", "options.values(1).include?(:name)"):
        add("Performance/InefficientHashSearch",source)
    for source in ("items.reverse.each { |item| p item }", "items.reverse.each", "result = items.reverse.each", "p(items.reverse.each)", "items.reverse.each(2)", "items&.reverse&.each { |item| p item }", "def run; items.reverse.each; :done; end", "items.reverse().each()"):
        add("Performance/ReverseEach",source)
    add("Performance/Caller", "caller(9223372036854775808).first")
    add("Performance/Caller", "caller(010)[02]")
    for cop, source in (("Performance/Caller", "caller.first"), ("Performance/InefficientHashSearch", "options.keys.include?(:name)"), ("Performance/ReverseEach", "items.reverse.each")):
        add(cop, source, AutoCorrect="disabled")
    add("Rails/OutputSafety", "t.html_safe")
    add("Rails/OutputSafety", "t = value\nt.html_safe")
    add("Rails/OutputSafety", "raw(input) { I18n.t(:name) }")
    add("FactoryBot/FactoryClassName", "factory :foo, { :\"cl\\x61ss\" => Foo }")
    add("FactoryBot/FactoryClassName", "factory :foo, { 'class': Foo }")
    add("FactoryBot/AttributeDefinedStatically", "factory :user do\n account 'factory': :account\nend")
    add("Performance/UriDefaultParser", "URI::Parser.new( # keep this\n)")
    add("Performance/UriDefaultParser", "URI::Parser.new { # keep this\n foo }")
    add("Performance/Caller", "caller.first { # keep this\n foo }")
    for source in ("scope :active, where(active: true)", "scope(:active, User.where(active: true))", "scope :active, query", "query = User.where(active: true)\nscope :active, query", "scope :active, -> { where(active: true) }", "self.scope :active, where(active: true)", "scope :active, User&.where(active: true)", "scope :active, where { |x| x.active? }"):
        add("Rails/ScopeArgs",source)
    for source in ("assert !valid?", "assert(!valid?)", "assert( ! valid?)", "assert !valid?, 'message'", "self.assert !valid?", "assert_not valid?", "assert !!valid?", "assert !value.ready?", "assert  !   valid?"):
        add("Rails/AssertNot",source)
    for base in ("ApplicationRecord", "ActiveModel::Base", "ActiveRecord::Base", "::ApplicationRecord", "Other"):
        for method in ("save", "update", "destroy", "create"):
            add("Rails/ActiveRecordOverride", f"class User < {base}\n def {method}\n  super\n end\nend")
    for source in ("class User < ApplicationRecord\n def save; super(); end\nend", "class User < ApplicationRecord\n def self.save; super; end\nend", "class User < ApplicationRecord\n def save; super(value); end\nend", "class User < ApplicationRecord\n def save; yield; end\nend"):
        add("Rails/ActiveRecordOverride",source)
    for source in ("has_and_belongs_to_many :roles", "has_and_belongs_to_many(:roles)", "self.has_and_belongs_to_many :roles", "model.has_and_belongs_to_many :roles", "has_and_belongs_to_many", "has_and_belongs_to_many = value\nhas_and_belongs_to_many"):
        add("Rails/HasAndBelongsToMany",source)
    for source in ("method.bind(object).call", "method.bind(object).call()", "method.bind(object).call(foo,bar)", "method.bind(object).(foo)", "bind(object).call(foo)", "method.bind(object).call { work }", "method.bind(object).call(&:name)", "method.bind(object,other).call", "method.bind.call", "method&.bind(object).call", "method.bind(object)&.call", "method.bind(object) { work }.call", "method.bind(object).call(foo: 1)", "method.bind(object).call(*args)"):
        add("Performance/BindCall", source)
    add("Performance/BindCall", "method.bind(object).call(foo)", AutoCorrect="disabled")
    for source in ("String.new", "String.new()", "String.new('text')", 'String.new("#{value}")', "::String.new", "Other::String.new", "String.new(value)", "String.new('text', encoding: 'UTF-8')", "String.new { work }", "String&.new", "String.to_s", "''.clone", "'text'.dup", "String.new.upcase", "String.new[0]", "String.new == value", "!String.new", "consume(String.new)", "[String.new]", "{value: String.new}", "String.new + other", "String.new('a').dup"):
        add("Performance/UnfreezeString", source)
    add("Performance/UnfreezeString", "String.new", AutoCorrect="disabled")
    for version in (2.2, 2.3, 2.6, 2.7, 3.2, 3.3, 3.4):
        add("Performance/BindCall", "method.bind(object).call(foo)", TargetRubyVersion=version)
        add("Performance/UnfreezeString", "'text'.dup", TargetRubyVersion=version)
        add("Performance/UnfreezeString", "String.new('text')", TargetRubyVersion=version)
    for source in ("text.start_with?('a') || text.start_with?('b')", "text.start_with?('a', 'b') || text.start_with?('c')", "text.end_with?('a') || text.end_with?(CONST)", "text.start_with?('a') || other.start_with?('b')", "text.start_with?('a') || text.end_with?('b')", "text&.start_with?('a') || text&.start_with?('b')", "start_with?('a') || start_with?('b')", "text.start_with?('a') || text.start_with?(value)", "value = 'b'; text.start_with?('a') || text.start_with?(value)", "text.start_with?('a') || text.start_with?(@value)", "text.start_with?('a') || text.start_with?(false)", "text.start_with?('a') || text.start_with?(['b'])", "text.start_with?('a') || text.start_with?(%Q(#{value}))", "text.start_with?('a') || text.start_with?(%Q(#{@value}))", "text.start_with?('a') or text.start_with?('b')", "text.starts_with?('a') || text.starts_with?('b')", "text.start_with?('a') || text.start_with?('b') || text.start_with?('c')", "text.start_with?('a') || text.start_with?", "text.start_with? || text.start_with?('a')"):
        add("Performance/DoubleStartEndWith", source)
    add("Performance/DoubleStartEndWith", "text.starts_with?('a') || text.starts_with?('b')", IncludeActiveSupportAliases=True)
    add("Performance/DoubleStartEndWith", "text.ends_with?('a') || text.ends_with?('b')", IncludeActiveSupportAliases=True)
    add("Performance/DoubleStartEndWith", "text.start_with?('a') || text.start_with?('b')", AutoCorrect="disabled")
    for arg in ("-1", "+1", "1.5", ":value", "::Namespace::VALUE", "%w[a b]", "defined?(value)", "(true ? 'a' : 'b')", "(/prefix/)", "[value]", "{key: @value}"):
        add("Performance/DoubleStartEndWith", f"text.start_with?('a') || text.start_with?({arg})")
    for left, right in (("'text'", '\"text\"'), ("object(1)", "object(0x1)"), ("object('a')", 'object(\"a\")'), ("object(1)", "object(2)")):
        add("Performance/DoubleStartEndWith", f"{left}.start_with?('a') || {right}.start_with?('b')")
    for left, right in (("object", "object()"), ("object 1", "object(1)"), ("object(1.0)", "object(1e0)")):
        add("Performance/DoubleStartEndWith", f"({left}).start_with?('a') || ({right}).start_with?('b')")
    for selector in ("start_with?", "end_with?"):
        for arg in ("'b'", "@value", "CONST", "defined?(value)", "(true ? 'a' : 'b')", "%w[a b]", "-1"):
            add("Performance/DoubleStartEndWith", f"text = 'beta'; text.{selector}('a') || text.{selector}({arg})")
    for source in ("enum :status, { active: 0, archived: 0 }", "enum :status, [:active, :archived, :active]", "enum status: { active: 0, archived: 0 }", "enum status: [:active, :archived, :active]", "enum :status, [0, 1, 0, 1, 0]", "enum :status, [1, 0x1, 01, 1_0, 10]", "enum :status, ['a', \"a\", 'b']", "enum :status, [1.0, 1e0]", "enum :status, [value, value(), other]", "enum :status, [foo(1), foo(0x1)]", "enum :status, %i[active archived active]", "enum :status, %w[active archived active]", "enum :status, {active: 0, **other, archived: 0}", "enum status: [0, 0], other: [1, 1]", "enum :status, [0, 1]", "model.enum :status, [0, 0]", "enum :status, VALUES", "enum :status, [0, 0], prefix: true"):
        add("Rails/EnumUniqueness", source)
    for source in ("enum :status, [nil, nil, true, true, false, false]", "enum 'état', ['été', 'été']", "enum :status, [:active, :\"active\"]", "enum :status, [-1, -0x1, +1, 1]", "enum :status, [[1], [0x1], {key: 1}, {key: 1}]", "enum :status, [0, # keep\n 0]", "enum({status: [0, 0]})", "enum status: {active: value, archived: value()}"):
        add("Rails/EnumUniqueness", source)
    for source in ("enum :status, [:active, :archived]", "enum status: [:active, :archived]", "enum({status: [:active, :archived]})", "enum :status, []", "enum :status, [0, 1]", "enum :status, ['a', 'b']", "enum :status, ['été', '😀']", "enum :status, [:été, :'foo bar']", "enum :status, %i[active archived]", "enum :status, %w[active archived]", "enum :status, %w[a\\ b c\\nd]", "enum :status, %W[a\\nb c]", "enum :status, [value, call()]", "enum :status, [\"#{value}\"]", "enum :status, %W[a#{value}]", "enum :status, [:a, # keep\n :b]", "enum :status, [:active], prefix: true", "enum :status, {active: 0}", "model.enum :status, [:active]", "enum status: [:active], other: [:done]"):
        add("Rails/EnumHash", source)
    for version in (6.1, 7.0, 7.2, 8.1):
        add("Rails/EnumHash", "enum :status, [:active]", TargetRailsVersion=version)
        add("Rails/EnumHash", "enum status: [:active]", TargetRailsVersion=version)
    add("Rails/EnumHash", "enum :status, [:active]", AutoCorrect="disabled")
    for source in ("enum :status, [?a, ?b]", "enum :status, [\"\\0\\a\\b\\e\\f\\r\\t\\v\"]", "enum :status, ['#{value}', '#@value', '#$value']", "enum :status, [:'a-b', :'été 😀']", "enum :status, %i[a\\ b c\\nd]", "enum :status, %I[a\\nb c]"):
        add("Rails/EnumHash", source)
    for source in ("enum :status, [:'😀', :'!@', :'~@', :'+@', :'-@', :'[]']", "enum :status, [?\\n, ?\\t, ?\\\\, ?\\\"]", "enum :status, %W[a\\ b c\\]d]", "enum :status, %i[a\\]b c]"):
        add("Rails/EnumHash", source)
    for source in ("enum :status, [{a: 1}, {:a => 1}]", "enum :status, [{\"a\": 1}, {:a => 1}]", "enum :status, [{a: 1}, {'a' => 1}]", "enum :status, [foo(a: 1), foo({a: 1})]", "enum :status, [[1], [1,]]", "enum :status, [foo(proc { work }), foo(proc do work end)]", "enum :status, [foo(proc {}), foo(proc do end)]", "enum :status, [/a/, /a/i]", "enum :status, [1..2, 1...2]"):
        add("Rails/EnumUniqueness", source)
    for source in ("enum :status, [- 1, -1, + 1, 1]", "enum :status, [- -1, 1]", "enum :status, [~1, -2]"):
        add("Rails/EnumUniqueness", source)
    add("Performance/Caller", "caller(+ 1).first")
    return rows
