"""Pinned original-cop branch specimens for the framework parity queue."""
import json
from pathlib import Path

def cases():
    rows = []
    def add(cop, source, **config):
        if cop in ("Rails/IndexWith", "Rails/ContentTag"):
            config.setdefault("GemVersions", {"railties": "8.1.3.1"})
        if cop == "Rails/UnknownEnv":
            config.setdefault("Environments", ["production", "development", "test", "staging", "promptfoo"])
        rows.append(dict(id=f"{cop}:{len(rows)}", cop=cop, path="app/helpers/example.rb" if cop=="Rails/ContentTag" else "app/models/example.rb", source=source + "\n", expected=[], corrected=source + "\n", config=config))
    for body in ("(has_many :children)", "begin; has_many :children; end", "if flag; has_many :children; end", "(has_many :children do; work; end)"):
        for prefix in ("", "work; "):
            add("Rails/HasManyOrHasOneDependent",f"class Model; with_options dependent: :destroy do; {prefix}{body}; end; end")
    for ruby in (2.5,2.7,3.4):
        for source in ("with_options dependent: :destroy do; _1.has_many :children; end", "with_options dependent: :destroy do; has_many :children do; _1; end; end"):
            add("Rails/HasManyOrHasOneDependent",source,TargetRubyVersion=ruby)
    add("Rails/ContentTag", "tag(<<~NAME)\nbr\nNAME")
    add("Rails/ContentTag", "tag(:br, data: <<~HTML)\n#{tag(:br)}\nHTML")
    add("Rails/ContentTag","tag(:br, data: {nested: tag(:br)})",AutoCorrect="disabled")
    add("Rails/ContentTag","tag( # rubocop:disable Rails/ContentTag\n :br, data: {nested: tag(:br)})")
    add("Rails/ContentTag","tag(1, data: {nested: tag(:br)})")
    add("Rails/ContentTag","object.tag(:br, data: {nested: tag(:br)})")
    for row in json.loads((Path(__file__).parents[2] / "tests/fixtures/framework-content-tag-original-spec.json").read_text())["cases"]:
        add("Rails/ContentTag",row["source"].rstrip("\n"),**row["config"])
    for value in (":br", ":p", ":hr", ":'foo-bar'", "'foo_bar'", ":_foo", "'BR'", "'br\nx'", "''", ":\"\"", "nil", "true", "false", "1", "-1", "1.5", "1r", "1i", "/x/", "[]", "{}", "name", "@name", "CONST", "provider.name", '"#{name}"', '"b" "r"', "?b", "__LINE__", "__FILE__", "(:br)", "(1)"):
        for rest in ("", ", class: 'x'", ", class: 'x', id: 'y'", ", {class: 'x'}", ", nil", ", options", ", **options", ", &block", ", escape: false", ", {}, true"):
            add("Rails/ContentTag",f"tag({value}{rest})")
    for name in ("area","base","br","col","embed","hr","img","input","keygen","link","meta","source","track","wbr"):
        add("Rails/ContentTag",f"tag(:{name}, class: 'x')")
    for version in (4.2,5.0,5.1,8.1):
        for gem in ({}, {"railties":"5.0.9"}, {"railties":"5.1.0"}, {"railties":"8.1.3.1"}):
            add("Rails/ContentTag","tag(:br)",TargetRailsVersion=version,GemVersions=gem)
    for source in ("tag :br", "self.tag(:br)", "object.tag(:br)", "object&.tag(:br)", "tag(:br, data: {nested: tag(:br)})", "tag(:p, data: {nested: tag(:br)})", "tag(:br) { tag(:div) }", "tag(:div) { work }", "tag(:br, # keep\nclass: 'x')", "tag(:br, class: # keep\n'x')", "tag(:br,\n class: __LINE__)", "tag(:br,\n class: 'x'); p __LINE__", "Nokogiri::XML.fragment(tag(:br))", "Nokogiri.XML(tag(:br))", "REXML::Document.new(tag(:br))", "tag +1", "tag +1, options", "tag +1.5", "tag +1r", "tag +\"br\"", "tag +name", "tag +1 +2", "tag(+ 1)", "tag(- 1)", "tag(:br) = value"):
        add("Rails/ContentTag",source)
    add("Rails/ContentTag","tag(:br)",AutoCorrect="disabled")
    for row in json.loads((Path(__file__).parents[2] / "tests/fixtures/framework-string-values.json").read_text())["cases"]:
        if "<<" not in row["source"]:
            value=row["source"][6:-2]
            key=json.dumps(row["value"]+"_id",ensure_ascii=False).replace("#{",chr(92)+"#{").replace("#@",chr(92)+"#@").replace("#$",chr(92)+"#$")
            add("Rails/RedundantForeignKey",f"class Post; has_many :comments, as: {value}, foreign_key: {key}; end")
    for value,key in (("1r","1/1_id"),("1.5r","3/2_id"),("-1r","-1/1_id"),("0.0r","0/1_id"),("1i","0+1i_id"),("/a/","(?-mix:a)_id")):
        add("Rails/RedundantForeignKey",f"class Post; has_many :comments, as: {value}, foreign_key: {json.dumps(key)}; end")
    for row in json.loads((Path(__file__).parents[2] / "tests/fixtures/framework-foreign-key-original-spec.json").read_text())["cases"]:
        add("Rails/RedundantForeignKey",row["source"].rstrip("\n"),**row["config"])
    for wrapper in ("{}", "class Post; {}; end", "class Admin::Post; {}; end", "module Admin; class HTTPServer; {}; end; end", "module Admin; {}; end", "class Post; ordinary {{ {} }}; end", "class Post; items.each {{ _1; {} }}; end", "Post.class_eval {{ {} }}", "class Post; class_eval {{ {} }}; end", "object.class_eval {{ {} }}", "Post = Class.new {{ {} }}", "Post = Class.new(Base) {{ {} }}", "Post = Module.new {{ {} }}", "Post = Other::Class.new {{ {} }}", "Other::Post = Class.new {{ {} }}", "class << Post; {}; end", "class Post; class << self; {}; end; end", "class Admin::Post; class << self; {}; end; end", "class << object; {}; end", "class factory::Post; {}; end", "Post = Struct.new {{ {} }}"):
        for key in ("post_id", "http_server_id", "admin_id", "object_id", "#<class:post>_id", "post>_id"):
            for macro in ("has_many", "belongs_to"):
                add("Rails/RedundantForeignKey",wrapper.format(f"{macro} :post, foreign_key: {json.dumps(key)}"))
    for macro in ("belongs_to", "has_many", "has_one", "has_and_belongs_to_many"):
        for source in (f"class Post; {macro} :post, foreign_key: 'post_id'; end", f"class Post; self.{macro} :post, foreign_key: 'post_id'; end", f"class Post; object&.{macro} :post, foreign_key: 'post_id'; end", f"class Post; {macro} :post, -> {{ work }}, foreign_key: 'post_id'; end", f"class Post; {macro} :post, {{foreign_key: :post_id}}; end", f"class Post; {macro} :post, foreign_key: :post_id, dependent: :destroy; end", f"class Post; {macro} :post, dependent: :destroy, foreign_key: :post_id; end", f"class Post; {macro} :post, foreign_key: 'post_id', foreign_key: 'other_id'; end", f"class Post; {macro} :post, **{{foreign_key: 'other_id'}}, foreign_key: 'post_id'; end", f"class Post; {macro} :post, foreign_key: # keep\n'post_id'; end", f"class Post; {macro} :post, foreign_key: 'post_id' do; work; end; end"):
            add("Rails/RedundantForeignKey",source)
    for value in ("nil", "false", "true", ":owner", "'owner'", "''", "1", "-1", "1.0", "1e-20", "value", "CONST", "[]", "{}"):
        for key in ("post_id", "owner_id", "_id", "true_id", "1_id", "-1_id", "1.0_id", "1.0e-20_id"):
            add("Rails/RedundantForeignKey",f"class Post; has_many :comments, as: {value}, foreign_key: {json.dumps(key)}; end")
    add("Rails/RedundantForeignKey","class Post; belongs_to :post, foreign_key: 'post_id'; end",AutoCorrect="disabled")
    for row in json.loads((Path(__file__).parents[2] / "tests/fixtures/framework-dependent-original-spec.json").read_text())["cases"]:
        add("Rails/HasManyOrHasOneDependent",row["source"].rstrip("\n"),**row["config"])
    for macro in ("has_many", "has_one"):
        for receiver in ("", "self.", "object.", "object&."):
            for options in ("", ", dependent: nil", ", dependent: false", ", dependent: :destroy", ", through: nil", ", through: false", ", through: :other", ", class_name: :Child", ", **options", ", **{dependent: :destroy}", ", **{}, dependent: :destroy", ", **options, dependent: nil", ", 'dependent' => :destroy", ", {dependent: :destroy}", ", {}"):
                add("Rails/HasManyOrHasOneDependent", f"class Model; {receiver}{macro} :children{options}; end")
    for body in ("has_many :children", "has_many :children; work", "work; has_many :children", "has_many :children do; work; end", "has_one :child do; work; end", "has_many :children do |x|; work; end", "has_many :children do; _1; end"):
        for wrapper in ("{}", "class Model; {}; end", "class Model < ActiveResource::Base; {}; end", "class Model < ::ActiveResource::Base; {}; end", "class Model < Other::ActiveResource::Base; {}; end", "with_options dependent: :destroy do; {}; end", "with_options dependent: :destroy do; work; {}; end", "with_options dependent: :destroy do; with_options class_name: 'Child' do; {}; end; end", "with_options dependent: :destroy do; work; with_options class_name: 'Child' do; {}; end; end", "object.with_options dependent: :destroy do; {}; end"):
            add("Rails/HasManyOrHasOneDependent", wrapper.format(body))
    for definition in ("def readonly? = true", "def readonly? = false", "def readonly? = (true)", "def readonly?; true; end", "def readonly?(); true; end", "def readonly?; false; end", "def readonly?(x); true; end", "def self.readonly?; true; end", "def readonly?; work; true; end", "def readonly?; (true); end", "def readonly?; true; ensure; work; end", "class Nested; def readonly?; true; end; end", "ActiveResource::Base"):
        add("Rails/HasManyOrHasOneDependent",f"class Model; has_many :children; {definition}; end")
    for source in ("class Model; has_many; end", "class Model; has_many dependent: :destroy; end", "class Model; has_many :children, -> { work }; end", "class Model; has_many :children, dependent:; end", "with_options({}) { has_many :children }", "with_options dependent: nil do |a,b|; a.has_many :children; end"):
        add("Rails/HasManyOrHasOneDependent",source)
    for row in json.loads((Path(__file__).parents[2] / "tests/fixtures/framework-inverse-of-original-spec.json").read_text())["cases"]:
        add("Rails/InverseOf",row["source"].rstrip("\n"),**row["config"])
    for macro in ("has_many", "has_one", "belongs_to"):
        for receiver in ("", "self.", "object.", "object&."):
            for options in ("", ", foreign_key: :owner_id", ", conditions: nil", ", conditions: false", ", foreign_key: nil", ", as: :owner", ", through: false", ", polymorphic: false", ", inverse_of: nil", ", inverse_of: false", ", foreign_key: :id, inverse_of: nil", ", foreign_key: :id, through: nil", ", foreign_key: :id, through: :owners", ", foreign_key: :id, inverse_of: false", ", 'foreign_key' => :id", ", **options"):
                for scope in ("", ", -> { where(flag: true) }", ", -> { _1 }"):
                    add("Rails/InverseOf",receiver+macro+" :children"+scope+options)
    for rails in (5.1,5.2,8.1):
        for ignore in (False,True):
            add("Rails/InverseOf","has_many :children, -> { where(flag: true) }, as: :owner",TargetRailsVersion=rails,IgnoreScopes=ignore)
            add("Rails/InverseOf","has_many :children, as: :owner",TargetRailsVersion=rails,IgnoreScopes=ignore)
    for header in ("with_options inverse_of: false do", "with_options inverse_of: false do ||", "with_options inverse_of: false do |assoc|", "with_options inverse_of: false do |_assoc|", "with_options inverse_of: false do |*assoc|", "object.with_options inverse_of: false do", "with_options foreign_key: :id do", "with_options inverse_of: false do"):
        for invocation in ("has_many :children, -> { work }", "assoc.has_many :children, foreign_key: :id", "_assoc.has_many :children, foreign_key: :id", "self.has_many :children, foreign_key: :id", "other { has_many :children, -> { work } }", "with_options foreign_key: :id do; has_many :children; end"):
            add("Rails/InverseOf",header+"; "+invocation+"; end")
    for source in ("with_options(inverse_of: false) { _1.has_many :children, foreign_key: :id }", "has_many :children, ->(x) { x }", "has_many :children, proc { work }", "has_many :children, scope { work }", "has_many :children, -> { work }, inverse_of:", "has_many({foreign_key: :id})", "has_many(foreign_key: :id, conditions: true)", "has_many :children, {foreign_key: :id}", "has_many :children, {foreign_key: :id}, {inverse_of: false}", "has_many :children, foreign_key: :id, inverse_of: nil, inverse_of: false", "has_many :children, foreign_key: :id do; work; end"):
        add("Rails/InverseOf",source)
    for ruby in (2.5,2.7,3.4):
        for scope in ("-> { _1 }", "proc { _1 }", "scope { _1 }", "->(x) { x }", "-> { value }"):
            add("Rails/InverseOf",f"has_many :children, {scope}",TargetRubyVersion=ruby)
    for key in (":foreign_key => :id", "'foreign_key' => :id", '"foreign_key": :id', ":'foreign_key' => :id", "foreign_key:"):
        add("Rails/InverseOf",f"has_many :children, {key}")
    for header in ("with_options inverse_of: false do |assoc = nil|", "with_options inverse_of: false do |**assoc|", "with_options inverse_of: false do |&assoc|", "with_options inverse_of: false do |(assoc,other)|", "with_options inverse_of: false do |assoc, other|", "with_options inverse_of: false do |; assoc|"):
        for recv in ("assoc.", ""):
            add("Rails/InverseOf",header+"; "+recv+"has_many :children, foreign_key: :id; end")
    for row in json.loads((Path(__file__).parents[2] / "tests/fixtures/framework-dynamic-find-original-spec.json").read_text())["cases"]:
        add("Rails/DynamicFindBy",row["source"].rstrip("\n"),**row["config"])
    for receiver in ("User.", "user.", "User&.", "", "page.", "Gem::Specification.", "::Gem::Specification."):
        for method in ("find_by_name", "find_by_name!", "find_by_name_and_email", "find_by_sql", "find_by_token_for", "find_by_", "find_by__and_name", "find_by_name_and_", "find_by_name_and_and_email"):
            for args in ("(name)", "(name, email)", "({name: name})", "(*names)", "(name: name)", "(name, &block)"):
                add("Rails/DynamicFindBy", receiver+method+args)
    for parent in ("ApplicationRecord", "::ApplicationRecord", "ActiveRecord::Base", "::ActiveRecord::Base", "Other::ApplicationRecord", "Other::ActiveRecord::Base", "ActiveModel::Base", "(ApplicationRecord)", "factory(ApplicationRecord)"):
        for body in ("find_by_name(name)", "def call; find_by_name(name); end", "class Inner; find_by_name(name); end", "find_by_name(name) { work }"):
            add("Rails/DynamicFindBy",f"class User < {parent}; {body}; end")
    for option in ("AllowedMethods", "AllowedReceivers", "Whitelist"):
        for values in ([], ["find_by_name"], ["User"], None):
            add("Rails/DynamicFindBy", "User.find_by_name(name)", **{option: values})
    for source in ("class User < ApplicationRecord; find_by__and_; end", "User.find_by__and_", "User.find_by_name_and_name(first, second)", "User.find_by_name name", "User.find_by_name(name) { work }", "User.find_by_name(name) do |x| work; end", "User.find_by_name_and_email(name, # keep\nemail)", "User.find_by_name_and_email(name, **options)", "User.find_by_name(name, &block)", "User.find_by_name(&block)", "User.find_by_name(name) = value", "User.find_by_name_and_email((name), (email))", "User.find_by_é(value)", "User.find_by_Foo(value)"):
        add("Rails/DynamicFindBy",source)
    add("Rails/DynamicFindBy","User.find_by_name(name)", AutoCorrect="disabled")
    for left in ("true", "false", "nil", "1", "2.0", "'value'", ":value", "first", "first()", "self.first", "@first", "CONST", "(condition ? true : false)", "!first", "first&.flag"):
        for right in ("true", "false", "nil", "2", "3.0", "'other'", ":other", "second", "self.second", "@second"):
            add("Rails/RedundantAllowNil",f"validates :title, allow_nil: {left}, allow_blank: {right}")
    for source in ("validates :title, presence: true", "validates :title, allow_nil: true", "validates :title, allow_blank: true", "validates :title, length: {is: 5}, allow_blank: false, allow_nil: false", "validates :title, allow_nil: false, allow_blank: true, length: {is: 5}", "validates :title, format: {with: /regexp/, allow_nil: true, allow_blank: true}", "validates :title, format: {allow_blank: true}, presence: {allow_nil: true}", "validates :title, {allow_nil: true, allow_blank: true}", "other.validates :title, allow_nil: true, allow_blank: true", "other&.validates :title, allow_nil: true, allow_blank: true", "validates :title, :allow_nil => true, :allow_blank => true", "validates :title, \"allow_nil\": true, \"allow_blank\": true", "validates :title, 'allow_nil' => true, 'allow_blank' => true", "validates :title, allow_nil: true, # keep\n allow_blank: true", "validates :title, allow_blank: true, # keep\n allow_nil: true", "validates :title, allow_nil: true, allow_blank: true, **opts", "validates :title, **opts, allow_nil: true, allow_blank: true", "validates :title, allow_nil: true, allow_nil: false, allow_blank: true", "allow_nil=true; allow_blank=true; validates :title, allow_nil:, allow_blank:", "validates(:title) { {allow_nil: true, allow_blank: true} }", "validates :title, custom: -> { {allow_nil: true, allow_blank: true} }"):
        add("Rails/RedundantAllowNil",source)
    for left,right in (("[]","%w[a]"),("{}","{a: 1}"),(":\"a#{value}\"",":\"b#{other}\""),("\"a#{value}\"","'a' 'b'"),("[1]","[2]"),("(first && other)","(first and other)"),("(first || other)","(first or other)"),("(condition ? true : false)","(condition ? false : true)"),("-> { 1 }","proc { 1 }"),("items.map { _1 }","items.map { |x| x }")):
        add("Rails/RedundantAllowNil",f"validates :title, allow_nil: {left}, allow_blank: {right}")
    add("Rails/RedundantAllowNil", "validates :title, allow_nil: true, allow_blank: true",AutoCorrect="disabled")
    for body in ("m.foo", "m.foo(1)", "m.foo(m.bar)", "m.foo; m.bar", "m.foo +1", "m.foo + 1", "m.foo && m.bar", "m.foo or m.bar", "m.foo(@value)", "m.foo(self)", "m.foo(m)", "m&.foo; m.bar", "m[0]", "m + 1", "!m", "-1; m.foo", "-1.5; m.foo", "__FILE__; m.foo", "__LINE__; m.foo", "__ENCODING__; m.foo", "m.foo = 1", "m.foo ||= 1", "m.foo(m:)", "m.foo { m.bar }", "m.map { _1 }", "m.map { m.foo(_1) }", "def nested(m); m.foo; end", "m.foo; class C; end", "m.foo; @state=1", "client.foo(m.bar)", "foo(m.bar)", "m.foo(value)"):
        add("Rails/RedundantReceiverInWithOptions",f"with_options(flag: true) {{ |m| {body} }}")
    for body in ("m.foo(other:)", "other=1; m.foo(other:)", "m.foo(User:)", "m.foo(_other:)", "_other=1; m.foo(_other:)", "m.foo(__LINE__:)", "m.foo(étiquette:)", "étiquette=1; m.foo(étiquette:)"):
        add("Rails/RedundantReceiverInWithOptions",f"with_options(flag: true) {{ |m| {body} }}")
    add("Rails/RedundantReceiverInWithOptions", "with_options(flag: true) { |_m| _m.foo(_m:) }")
    for selector in ("puts", "p", "__method__", "binding", "method_missing", "respond_to_missing?", "initialize"):
        add("Rails/RedundantReceiverInWithOptions",f"with_options(flag: true) {{ |m| m.{selector}(1) }}")
    for params in ("m", "m, other", "m = nil", "*m", "m; local", "(m,other)", "&m"):
        add("Rails/RedundantReceiverInWithOptions",f"with_options(flag: true) do |{params}|\n m.foo(1)\nend")
    for source in ("with_options(flag: true) do; end", "with_options(flag: true) { other.foo }", "with_options(flag: true) { 1 }", "with_options(flag: true) { |m| 1 }", "object.with_options(flag: true) { |m| m.foo }", "with_options(flag: true) do |m|\n m.foo\n m.bar\nend", "foo = :local; with_options(flag: true) { |m| m.foo }", "foo = :local; with_options(flag: true) { |m| m.foo() }", "with_options(flag: true) { |m| m.super }", "with_options(flag: true) { |m| m.nil }", "with_options(flag: true) { |m| m.__FILE__ }", "with_options(flag: true) { |m| m.foo # keep\n}", "with_options(flag: true) do |m|\n m.foo # rubocop:disable Rails/RedundantReceiverInWithOptions\n m.bar\nend"):
        add("Rails/RedundantReceiverInWithOptions",source)
    for ruby in (2.5,2.7,3.4):
        for body in ("_1.foo", "_1.foo(_1)", "_1.foo(_2)", "_1.foo; _1.bar", "_1.foo(@value)", "_1.foo(self)"):
            add("Rails/RedundantReceiverInWithOptions",f"with_options(flag: true) {{ {body} }}",TargetRubyVersion=ruby)
    add("Rails/RedundantReceiverInWithOptions", "with_options(flag: true) { |m| m.foo }", AutoCorrect="disabled")
    for method in ("after_action", "append_after_action", "append_around_action", "append_before_action", "around_action", "before_action", "prepend_after_action", "prepend_around_action", "prepend_before_action", "skip_after_action", "skip_around_action", "skip_before_action", "skip_action_callback"):
        for option in ("only", "except"):
            for value in (":missing", "'index'", "%i[index show missing]", "%I[index show]", "[:index, 'missing', value, nil]"):
                add("Rails/LexicallyScopedActionFilter", f"class C < ApplicationController; {method} :authorize, {option}: {value}; def index; end; def show; end; end")
    for source in ("before_action :authorize, only: :missing", "class C; before_action :authorize, only: :missing; end", "class C; other.before_action :authorize, only: :missing; end", "class C; before_action :authorize, only: :missing, if: :condition; end", "class C; before_action :authorize, {only: :missing}; end", "class C; before_action :authorize, only: []; end", "class C; before_action :authorize, only: :index; def self.index; end; end", "class C; before_action :authorize, only: :index; private def index; end; end", "class C; before_action :authorize, only: :index; if condition; def index; end; end; end", "class C; before_action :authorize, only: %i[index show]; def index; end; alias_method :show, :index; end", "class C; before_action :authorize, only: %i[index show]; def index; end; alias show index; end", "class C; before_action :authorize, only: %i[index show second]; def index; end; alias_method :show, :index; alias_method :second, :index; end", "class C; before_action :authorize, only: %i[index show second]; def index; end; alias_method :show, :index; alias_method :second, :show; end", "class C; before_action :authorize, only: :show; def index; end; alias_method 'show', 'index'; end", "class C; before_action :authorize, only: :show; def index; end; alias_method :show, :index; alias_method 'other', :index; end", "class C; before_action :authorize, only: :show; def index; end; other.alias_method :show, :index; end", "module M; included { before_action :authorize, only: :index }; def index; end; end", "module M; included { before_action :authorize, only: :index; def index; end }; end", "class C; before_action :authorize, only: :index unless condition; def index; end; end", "class C; before_action :authorize, only: :index; class Nested; def index; end; end; end", "class C; class Nested; before_action :authorize, only: :index; end; def index; end; end", "class C < (Base); before_action :authorize, only: :index; def index; end; end", "class C; (before_action :authorize, only: :index; def index; end); end", "class C; before_action :authorize, only: [:missing, :missing, :other]; end", "class C; before_action :authorize, only: :\"\"; end", "class C; before_action :authorize, only: \"in#{name}dex\"; end", "class C; before_action :authorize, only: 'in' 'dex'; end", "class C; before_action :authorize, only: ?i; end", "class C; before_action :authorize, only: :missing; alias_method; end", "class C; before_action :authorize, only: :missing; alias_method :new, old; end", "class C; before_action :authorize, only: :index do; end; def index; end; end"):
        add("Rails/LexicallyScopedActionFilter",source)
    for value in (r"%i[index sh\ ow]", r"%I[index sh\u006fw]", r"%w[index sh\ ow]", r"%W[index sh\u006fw]", r"%I[index #{value}]", r"%W[index #{value}]", '"a\nb"', '"index\\n"', ":\"a\nb\"", "%s()"):
        add("Rails/LexicallyScopedActionFilter",f"class C; before_action :authorize, only: {value}; def index; end; end")
    for source in ("module M; extend ActiveSupport::Concern; included do; before_action proc { authenticate }, only: :foo; end; def foo; end; end", "class C; before_action :authorize, {only: :missing, except: :other}; end", "class C; before_action :authorize, only: :missing, **options; end", "class C; before_action [:first, :second], only: :missing; end", "class C; before_action :first, :second, only: :missing; end", "class C; before_action :authorize, only: value; end", "class C; before_action :authorize, only:; end", "class C; before_action :authorize, only: :show; def index; end; alias_method :show, 'index'; end", "class C; before_action :authorize, only: :show; def index; end; alias :show :index; end", "class C; before_action :authorize, only: :missing do; work; end; end", "module M::N; before_action :authorize, only: :missing; end"):
        add("Rails/LexicallyScopedActionFilter", source)
    add("Rails/LexicallyScopedActionFilter", "class C; before_action :authorize, only: :missing; end", AutoCorrect="disabled")
    for cop, by in (("Rails/IndexBy", True), ("Rails/IndexWith", False)):
        for expr in ("foo(el)", "el.to_sym", "el", "h[el]", "foo(h)", "h.foo", "foo { h }", "'value'", "1"):
            pair=f"[{expr}, el]" if by else f"[el, {expr}]"
            assignment=f"h[{expr}] = el" if by else f"h[el] = {expr}"
            for source in (f"x.each_with_object({{}}) {{ |el, h| {assignment} }}", f"x.to_h {{ |el| {pair} }}", f"x.map {{ |el| {pair} }}.to_h", f"x.collect {{ |el| {pair} }}.to_h", f"Hash[x.map {{ |el| {pair} }}]", f"::Hash[x.map {{ |el| {pair} }}]", f"x&.each_with_object({{}}) {{ |el, h| {assignment} }}", f"x&.map {{ |el| {pair} }}&.to_h"):
                add(cop, source)
        pair="[el.to_sym, el]" if by else "[el, el.to_sym]"
        assignment="h[el.to_sym] = el" if by else "h[el] = el.to_sym"
        for source in (f"x.each_with_object({{}}) do |el, h|\n  {assignment}\nend", f"x.map {{ |el| {pair} }}.\n  to_h", f"x.map {{ |el| {pair} }}\n  .to_h", f"x.map {{ |el| {pair} }}.to_h {{ |k,v| [v,k] }}", f"Other::Hash[x.map {{ |el| {pair} }}]", f"Hash.[](x.map {{ |el| {pair} }})", f"x.map() {{ |el| {pair} }}.to_h()", f"x.to_h() {{ |el| {pair} }}", f"x.map(1) {{ |el| {pair} }}.to_h", f"x.map {{ |el| {pair} }}.to_h(1)", f"x.map {{ |el, other| {pair} }}.to_h", f"x.map {{ |el| work; {pair} }}.to_h", f"x.each_with_object({{a: 1}}) {{ |el,h| {assignment} }}", f"x.each_with_object({{}}) {{ |el,h,other| {assignment} }}", f"x.map {{ |el; other| {pair} }}.to_h", f"x.map {{ |el| {pair} }} # keep\n .to_h", f"x.each_with_object({{}}) {{ |el,h| # keep\n {assignment} }}"):
            add(cop,source)
        for ruby in (2.5,2.6,3.4):
            for rails in (5.2,6.0,8.1):
                add(cop,f"x.to_h {{ |el| {pair} }}",TargetRubyVersion=ruby,TargetRailsVersion=rails,GemVersions={"railties":str(rails)})
                add(cop,f"x.map {{ |el| {pair} }}.to_h",TargetRubyVersion=ruby,TargetRailsVersion=rails,GemVersions={"railties":str(rails)})
        for gems,target in (({},8.1), ({"railties":"5.2.8"},8.1), ({"railties":"8.1.3.1"},5.2)):
            add(cop,f"x.map {{ |el| {pair} }}.to_h",GemVersions=gems,TargetRailsVersion=target)
        for source in (f":: Hash[x.map {{ |el| {pair} }}]", f"Hash [x.map {{ |el| {pair} }}]", f"x.to_h {{ {pair} }}", f"x.map {{ [_1, _1.to_s] }}.to_h", f"x.each_with_object({{}}) {{ |el,h| h[el] = h }}", f"x.map {{ |el| [{('begin; break :halt; 1; end' if by else 'el')}, {('el' if by else 'begin; break :halt; 1; end')}] }}.to_h", f"x.map {{ |el| [{('begin; next; 1; end' if by else 'el')}, {('el' if by else 'begin; next; 1; end')}] }}.to_h"):
            add(cop,source)
        for source in (f"x.map {{ |el| {pair} }}.to_h = other", f"Hash[x.map {{ |el| {pair} }}] = other"):
            add(cop,source)
        pair_splat="[*other, el]" if by else "[el, *other]"
        add(cop,f"x.map {{ |el| {pair_splat} }}.to_h")
        add(cop,f"x.map {{ |el| {pair} }}.to_h",AutoCorrect="disabled")
    status_names = json.loads((Path(__file__).resolve().parents[2] / "tests/fixtures/framework-http-status-rack-2.2.24.json").read_text())
    for name, number in status_names.items():
        for style in ("symbolic", "numeric"):
            for value in (str(number), repr(str(number)), ":"+name if name != "(unused)" else ':"(unused)"'):
                add("Rails/HttpStatus", f"render :foo, status: {value}", EnforcedStyle=style)
    for method in ("render", "redirect_to", "head", "assert_response", "assert_redirected_to"):
        for value in ("200", ":ok", "'200'", "550", "'550'", "'oops'", "'200foo'", "'  +2_00rest'", ":error", ":success", ":missing", ":redirect", ":ng", ':""', '"a\nb"', "1.0", "nil", "value", "__LINE__"):
            source=f"{method} {value}" if method in ("head", "assert_response") else f"{method} status: {value}"
            for style in ("symbolic", "numeric"):
                add("Rails/HttpStatus", source, EnforcedStyle=style)
    for source in ("render json: {status: 200}", "render :foo, {status: 200}", "render :foo, status: 200, other: true", "render :foo, {status: 200}, extra", "assert_redirected_to '/some/path', {status: 301}, 'message'", "head '200'", "head(200, location: 'accounts')", "other.render status: 200", "render :foo, status: 200, status: 404", "render :foo, 'status' => 200", "render :foo, status: (200)", "render :foo, status: -200", "render :foo, status: +200", "render :foo, status: 0xC8", "render :foo, status: '2__00'", "render :foo, status: '200\\n'", "render :foo, status: ?2", "render :foo, status: '2' '00'", "render :foo, status: <<~CODE\n200\nCODE", "render :foo, status: 200 # keep", "render :foo, **opts, status: 200"):
        add("Rails/HttpStatus", source)
    for source in ("render({status: 200}, {status: 404})", "render({status: 200}, :extra)", "assert_redirected_to({status: 200}, {status: 404})", "assert_redirected_to({status: 200}, :extra)", "render 200", "render '200'", "render :foo, 200", "redirect_to({status: 200}, 'extra')"):
        add("Rails/HttpStatus", source)
    add("Rails/HttpStatus", "render status: 200", AutoCorrect="disabled")
    for method in ("second", "minute", "hour", "day", "week", "fortnight", "month", "year", "byte", "kilobyte", "megabyte", "gigabyte", "terabyte", "petabyte", "exabyte", "zettabyte"):
        for number in ("-1", "-1.0", "1", "1.0", "-23", "-0.5", "0", "0.125", "42"):
            for suffix in ("", "s"):
                add("Rails/PluralizationGrammar", f"{number}.{method}{suffix}.ago")
    for number in ("0x1", "01", "0b10", "+1", "1_000", "1e-5", "1e-4", "1e14", "1e15", "1.2345678901234567e20", "1e309", "-1e309", "-0.0", "__LINE__", "(1)", "(1.0)", "value", "1r", "1i"):
        for suffix in ("", "s"):
            add("Rails/PluralizationGrammar", f"{number}.day{suffix}")
    for source in ("2.day(1)", "2.day { work }", "2&.day", "1.days(&block)", "2.day = 1", "def day; end", "2.days", "1.day", "2.dayss", "2.millisecond", "- 1.days", "+ 1.days", "1.day; __LINE__.day"):
        add("Rails/PluralizationGrammar",source)
    for source in ("- 2.day", "- 1.0.days", "- 1.days(2)", "- 1.days { work }", "- -1.days", "-(1.days)", "- (1.days)", "-\n1.days", "- 1 .days", "- 0x1.days", "+ 1.0.days", "+ 1.days(2)"):
        add("Rails/PluralizationGrammar",source)
    add("Rails/PluralizationGrammar", "2.day", AutoCorrect="disabled")
    for method in ("has_many", "has_one", "belongs_to"):
        for value in ("Account", "Account.name", "Account.to_s", "Account.other", "Account.name(1)", "::Account", "A::Account", "A::Account.name", "value", "value.to_s", "value.name", "nil", "false", "1", ":Account", ':""', '"Account"', '"#{prefix}Account"', "(Account)"):
            add("Rails/ReflectionClassName", f"{method} :accounts, class_name: {value}")
    for source in ("belongs_to :account, -> { distinct }, class_name: Account", "has_many :accounts, class_name: Account, foreign_key: :account_id", "class_name = Account; has_many :accounts, class_name: class_name", "class_name = 'Account'; has_many :accounts, class_name: class_name", "class_name = 'Account'; has_many :accounts, class_name:", "class_name = Account; has_many :accounts, class_name:", "has_many :accounts, class_name:", "model.has_many :accounts, class_name: Account", "has_many(:accounts, {class_name: Account})", "has_many :accounts, -> {}, other, class_name: Account", "has_many :accounts", "has_many :accounts, class_name: Account.name { work }", "has_many :accounts, class_name: Account&.name", "has_many :accounts, class_name: %s()", "has_many :accounts, class_name: :\"a\nb\"", "has_many :accounts, class_name: 'A' 'B'", "has_many :accounts, class_name: Account, class_name: Other"):
        add("Rails/ReflectionClassName", source)
    for source in ("name = :Account; has_many :accounts, class_name: name", "name = :\"\"; has_many :accounts, class_name: name", "name = \"#{prefix}Account\"; has_many :accounts, class_name: name", "name = Account; has_many :accounts, class_name: name; name = 'Account'", "if flag; name = 'Account'; end; has_many :accounts, class_name: name", "has_many :accounts, class_name: Account, **options", "has_many :accounts, **options, class_name: Account", "has_many :accounts, {class_name: Account}, other", "has_many :accounts, class_name: to_s", "has_many :accounts, class_name: Account.to_s(1)", "has_many :accounts, class_name: Account.name(&block)", "has_many :accounts, class_name: Account # keep", "has_many :accounts, class_name: (::Account).name", "has_many :accounts, class_name: Account::Name", "has_many :accounts, class_name: 'a\nb'", "has_many :accounts, 'class_name' => Account", "has_many :accounts, :class_name => Account", "has_many :accounts, class_name: Account, class_name: Other.name", "has_many :accounts, class_name: false, class_name: Account", "has_many :accounts, class_name: :Account, class_name: Account", "has_many :accounts, class_name: /Account/"):
        add("Rails/ReflectionClassName", source)
    add("Rails/ReflectionClassName", "has_many :accounts, class_name: Account", AutoCorrect="disabled")
    for source in ("Rails.env.proudction?", "::Rails.env.developmen?", "Rails.env.local?", "Rails.env.production?", "Rails.env.unknown?", "Rails.env == 'proudction'", "'proudction' === Rails.env", "Rails.env.==('proudction')", "Rails.env != 'proudction'", "Rails.env == :proudction", "Rails.env&.proudction?", "Other::Rails.env.proudction?", "Rails.env(:x).proudction?", "Rails.env.proudction?(1)", "Rails.env.production", "Rails.env == \"production?\""):
        add("Rails/UnknownEnv",source)
    for version in (7.0,7.1):
        for source in ("Rails.env.local?", "Rails.env.lcoal?", "Rails.env == 'development'", "Rails.env.stagging?"):
            add("Rails/UnknownEnv",source,TargetRailsVersion=version)
            add("Rails/UnknownEnv",source,TargetRailsVersion=version,Environments=["local","staging","stage","staged"])
    for name in ("prodction","DEVELOPMENT","@@production","stagin","zzz","a","production?"):
        add("Rails/UnknownEnv",f"Rails.env == '{name}'",Environments=["development","production","staging","staging","stage","TEST"])
    for source in ("Rails.env == \"proudction?\"", "Rails.env == 'produc'", "Rails.env == \"proudction\\nfoo\"", "Rails.env == (\"proud\" \"ction\")", "Rails.env == :\"proudction\"", "Rails.env&.==('proudction')"):
        add("Rails/UnknownEnv",source)
    for dictionary in (None,[],["test","TEST","tEst","test"],["développement","dévelopement","development"]):
        for name in ("tset","developement","dévelopement", "LOCAL"):
            add("Rails/UnknownEnv",f"Rails.env == '{name}'",Environments=dictionary)
    for source in ("Rails.env.developpment?", "Rails.env.something?", "'developpment' == Rails.env", "'something' === Rails.env", "'local' === Rails.env", "Rails.env.local?", "Rails.env == 'local'", "Rails.env.production?", "Rails.env == 'production'"):
        add("Rails/UnknownEnv",source,TargetRailsVersion=7.0,Environments=["development","production","test"])
        add("Rails/UnknownEnv",source,TargetRailsVersion=7.1,Environments=None)
    for source in (":: Rails.env.proudction?", "Rails.env().proudction?()", "Rails.env == ?p", "Rails.env == <<~ENV\nproudction\nENV", "Rails.env == \"proudction\nfoo\"", "Rails.env == \"proudction\\\n\"", "Rails.env == \"pro#{name}duction\""):
        add("Rails/UnknownEnv",source)
    for source in ("Rails.env { value }.proudction?", "Rails.env.proudction? { value }", "Rails.env { value } == 'proudction'", "Rails.env.proudction?(&block)", "Rails.env(&block).proudction?"):
        add("Rails/UnknownEnv",source)
    for env in ("'production'", ":production", "'not-an-env'", "'empty'", "'nil'", "'production?'", "''", "\"pro#{name}duction\"", "(\"pro\" \"duction\")", ":\"prod\nfoo\""):
        for op in ("==", "!="):
            add("Rails/EnvironmentComparison",f"Rails.env {op} {env}")
            add("Rails/EnvironmentComparison",f"{env} {op} Rails.env")
    for source in ("Rails.env === 'production'", "Rails.env.==('production')", "Rails.env&.==('production')", ":: Rails.env != 'production'", "Other::Rails.env == 'production'", "Rails.env(1) == 'production'", "Rails.env { value } == 'production'", "Rails.env == ?p", "Rails.env == <<~ENV\nproduction\nENV", "Rails.env == # keep\n 'production'", "Rails.env.==('production') { work }"):
        add("Rails/EnvironmentComparison",source)
    add("Rails/EnvironmentComparison","Rails.env == 'production'",AutoCorrect="disabled")
    for version in (7.0,7.1):
        add("Rails/EnvironmentComparison","Rails.env == 'local'",TargetRailsVersion=version)
    for name in ("acts_like", "acts_like_string", "ascii_only", "between", "blank", "casecmp", "duplicable", "empty", "end_with", "ends_with", "eql", "equal", "exclude", "frozen", "html_safe", "in", "include", "instance_of", "instance_variable_defined", "is_a", "is_utf8", "kind_of", "match", "nil", "present", "respond_to", "start_with", "starts_with", "unicode_normalized", "valid_encoding"):
        add("Rails/EnvironmentComparison",f"Rails.env == '{name}'")
    for cop,anchor_start,anchor_end in (("Performance/DeletePrefix",r"\A",""),("Performance/DeleteSuffix","",r"\z")):
        for method in ("gsub","gsub!","sub","sub!"):
            for content in ("prefix",r"foo\.bar",r"a\/b",r"a\nb",r"a\tb",r"a\x41",r"a\x01",r"a\x7f",r"a\u2028",r"a\u0041",r"a\u{41}",r"a\cA",r"a\C-A","a'b",'a"b',r"a\\b",r"a\#b","été","",r"\d+","foo.*","foo|bar",r"\Afoo\z"):
                add(cop,f"str.{method}(/{anchor_start}{content}{anchor_end}/, '')")
            add(cop,f"str&.{method}(/{anchor_start}prefix{anchor_end}/, '')")
            add(cop,f"str.{method}(/{anchor_start}prefix{anchor_end}/, 'x')")
            add(cop,f"str.{method}(/{anchor_start}prefix{anchor_end}/i, '')")
            add(cop,f"str.{method}(/{anchor_start}prefix{anchor_end}/, '')",TargetRubyVersion=2.4)
            add(cop,f"str.{method}(/{anchor_start}prefix{anchor_end}/, '')",TargetRubyVersion=2.5)
        for safe in (True,False):
            pattern="^prefix" if anchor_start else "prefix$"
            add(cop,f"str.gsub(/{pattern}/, '')",SafeMultiline=safe)
            add(cop,f"str.gsub(/{anchor_start}prefix{anchor_end}/, '')",SafeMultiline=safe)
        for source in (f"str.sub(/{anchor_start}prefix{anchor_end}/, '') {{ work }}", f"str.sub(/{anchor_start}prefix{anchor_end}/, '')\np $~", f"str.sub(/{anchor_start}prefix{anchor_end}/, # keep\n '')"):
            add(cop,source)
        add(cop,f"str.sub(/{anchor_start}prefix{anchor_end}/, '')",AutoCorrect="disabled")
    for version in (2.5,3.0,3.1,3.4):
        for cop,pattern in (("Performance/DeletePrefix",r"\Aa\cA"),("Performance/DeleteSuffix",r"a\cA\z")):
            add(cop,f"str.sub(/{pattern}/, '')",TargetRubyVersion=version)
    add("Performance/DeletePrefix",r"Pathname.new('prefixbody').sub(/\Aprefix/, '')")
    add("Performance/DeleteSuffix",r"Pathname('bodysuffix').sub(/suffix\z/, '')")
    add("Rails/EnumHash", r'enum :status, [:"\u2028"]')
    add("Rails/EnumHash", r'enum :status, ["\u0001", "\u2028"]')
    add("Rails/EnumHash", r'enum :status, [:"a\u0001"]')
    for cop,start,end in (("Performance/StartWith",r"\A",""),("Performance/EndWith","",r"\z")):
        patterns=[start+"abc"+end]+[start+"\\"+letter+end for letter in "aefrtnv.*?$^|wWsSdDAZzGbBhHRXi jlmoqy".replace(" ","")]+[start+r"\\"+end]
        for pattern in patterns:
            for method in ("match", "match?", "=~"):
                for left,right in (("str",f"/{pattern}/"),(f"/{pattern}/","str")):
                    source=f"{left} =~ {right}" if method=="=~" else f"{left}.{method}({right})"
                    add(cop,source)
        for pattern in (start+"a.*"+end,start+"a|b"+end,start+""+end,start+r"a\cA"+end,start+r"a\x01"+end,start+r"a\u2028"+end):
            add(cop,f"str.match?(/{pattern}/)")
        pattern=start+"abc"+end
        for method in ("match", "match?", "=~"):
            expr=f"str =~ /{pattern}/" if method=="=~" else f"str.{method}(/{pattern}/)"
            for source in (f"if {expr}; work; end",f"work unless {expr}",f"while {expr}; break; end",f"!({expr})",f"{expr} ? yes : no",f"if {expr} && ready?; work; end",f"result = {expr} && ready?",f"result = ({expr})",f"str&.{method}(/{pattern}/)",f"/{pattern}/&.{method}(str)"):
                add(cop,source)
        for source in (f"str.match?(/{pattern}/i)",f"str.match?(/{pattern}/,offset)",f"match(/{pattern}/)",f"str.match?(/{pattern}/) {{ work }}",f"str.match?(/{pattern}/)\np $~",f"str.match(/{pattern}/)\np $~",f"/{pattern}/.match?(nil)",f"/{pattern}/.match?(foo || bar)",f"/{pattern}/.match?((foo || bar))",f"/{pattern}/.match?()",f"str.match?(/{start}a\nb{end}/)",f"str.match?(/{start}a\r\nb{end}/)"):
            add(cop,source)
        for safe in (True,False):
            pattern="abc$" if end else "^abc"
            add(cop,f"str.match?(/{pattern}/)",SafeMultiline=safe)
        add(cop,f"str.match?(/{start}abc{end}/)",AutoCorrect="disabled")
    add("Performance/EndWith",r"/abc\z/.match?()",AutoCorrect="disabled")
    for cop,pattern in (("Performance/StartWith",r"\Aabc"),("Performance/EndWith",r"abc\z")):
        for source in (f"str\r\n .match?(/{pattern}/)",f"str.match?(\r\n /{pattern}/)",f"if str =~\r\n /{pattern}/; work; end"):
            add(cop,source)
    for kind in ("acceptance","comparison","confirmation","exclusion","format","inclusion","length","numericality","presence","absence","size","uniqueness"):
        for args in (":name", ":name, :age", "[:name, :age]", "[:name, :age].freeze", "%i[name age]", "%i[name age].freeze", "%w[name age]", ":name, minimum: 1", ":name, {minimum: 1}"):
            add("Rails/Validation",f"validates_{kind}_of {args}")
        add("Rails/Validation",f"validates_{kind}_of(:name, :age)")
    for source in ("validates_presence_of", "validates_presence_of()", "self.validates_presence_of :name", "Other.validates_presence_of :name", "validates_presence_of attrs", "validates_presence_of ATTRS", "attrs = [:name]; validates_presence_of attrs", "validates_presence_of *attrs", "validates_presence_of :name, *attrs, :age, allow_nil: true", "validates_presence_of [:name, [:age]]", "validates_presence_of [:name, attributes]", "validates_presence_of []", "validates_presence_of %i[]", "validates_presence_of %W[#{field}]", "validates_presence_of %I[#{field}]", r"validates_presence_of %w[a\ b]", "validates_presence_of %w[é]", "validates_presence_of [:name, # keep\n :age]", "validates_presence_of [:name].freeze(&probe)", "validates_presence_of [:name].freeze(1)", "validates_presence_of ATTRS.freeze", "validates_presence_of 'name'.freeze", "validates_presence_of [:name]&.freeze", "validates_presence_of freeze", "validates_presence_of freeze()", "def configure(freeze); validates_presence_of freeze; end", "validates_presence_of :name, **options", "validates_presence_of :name, allow_nil: true, allow_blank: false", "validates_presence_of :name, :allow_nil => true", "validates_presence_of :name, 'allow_nil' => true", "validates_presence_of :name, # keep\n allow_nil: true", "validates_presence_of :name do; work; end", "validates_presence_of [:name].freeze do; work; end", "validates_presence_of :name, options.freeze", "validates_presence_of 1", "validates_presence_of -1", "validates_presence_of +1.5", "validates_presence_of ~1", "validates_presence_of 1..2", "validates_presence_of true", "validates_presence_of nil", "validates_presence_of /name/", "validates_presence_of :\"#{field}\"", "validates_presence_of \"#{field}\"", "validates_presence_of <<~ATTR\nname\nATTR", "def configure(*); validates_presence_of(*); end"):
        add("Rails/Validation",source)
    add("Rails/Validation","validates_presence_of :name",AutoCorrect="disabled")
    for source in ("validates_presence_of attrs", "validates_presence_of :name", "validates_presence_of", "puts :hello"):
        add("Rails/Validation",f"# rubocop:disable Rails/Validation\n{source}\n# rubocop:enable Rails/Validation")
    for argument in ("+1", "+1.5", "+1.2r", "+1i", "+name", '+"name"', "+(1)", "++1", "+1 +2", "+1..2", "+1 && ready", "+1 and ready", "+1 ? yes : no", "+1 rescue fallback"):
        add("Rails/Validation",f"validates_presence_of {argument}")
    for source in ("validates_presence_of(+1.5)", "wrapper(validates_presence_of +1, :name)", "!validates_presence_of +1.5", "self.validates_presence_of +1", "validates_presence_of = 1; validates_presence_of +1.5"):
        add("Rails/Validation",source)
    for source in ("validates_presence_of [:name, :age], allow_nil: true", "validates_presence_of fields, allow_nil: true", "validates_presence_of *[:name, :age]", "validates_presence_of *[:name, [:age]]", "validates_presence_of [:name], :age", "validates_presence_of :age, [:name]"):
        add("Rails/Validation",source)
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
    for cop, source, *options in PROGRAMS:
        runtime_config=dict(options[0]) if options else {}
        if cop=="Rails/ContentTag": runtime_config.setdefault("GemVersions",{"railties":"8.1.3.1"})
        rows.append(dict(id=f"{cop}:runtime:{len(rows)}", cop=cop, path="app/helpers/example.rb" if cop=="Rails/ContentTag" else "app/models/example.rb", source=source, expected=[], corrected=source, config=runtime_config, trusted_runtime=True))
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
    for key in ('type', ':""', '%s()', ':"a\nb"', '"a\nb"', '"a" "b"', '?a', '""'):
        for cop in ("Rails/EnumHash", "Rails/EnumUniqueness"):
            add(cop, f"enum {key}, [0, 0]")
    for value in (':""', '%s()', ':"a\nb"', '"a\nb"', '"a" "b"', '?a', '""'):
        add("Rails/EnumHash", f"enum :status, [{value}]")
    add("Performance/Caller", "caller(+ 1).first")
    for source in ("perform if value&.blank?", "perform unless value&.blank?", "if value&.blank?; perform; end", "unless value&.blank?; perform; else; other; end", "value&.blank? ? perform : other", "perform if (value&.blank?)", "perform if !value&.blank?", "perform if value.blank?", "perform if value&.blank?(extra)", "perform while value&.blank?", "if value&.blank?\n # keep\n perform\nend"):
        add("Rails/SafeNavigationWithBlank", source)
    add("Rails/SafeNavigationWithBlank", "perform if value&.blank?", AutoCorrect="disabled")
    for source in ("skip_before_action :login, only: :show, if: :trusted?", "skip_after_action :login, if: :trusted?, only: :show", "skip_around_action :login, except: :admin, if: :trusted?", "skip_action_callback :login, if: :trusted?, except: :admin", "skip_before_action :login, only: :show, except: :admin, if: :trusted?", "skip_before_action :login, only: :show", "skip_before_action :login, if: :trusted?", "controller.skip_before_action :login, only: :show, if: :trusted?", "before_action :login, only: :show, if: :trusted?", "skip_before_action :login, {if: :trusted?, only: :show}", "skip_before_action :login, {only: :show, if: :trusted?}", "skip_before_action :login, {'only' => :show, 'if' => :trusted?}", "skip_before_action :login, only: :show, if: -> { trusted? }", "skip_before_action :login, only: :show, if: compute", "skip_before_action :login, only: :show, if: :first, if: :last", "skip_before_action :login, only: :show, if: :trusted?, raise: false"):
        add("Rails/IgnoredSkipActionFilterOption", source)
    add("Rails/IgnoredSkipActionFilterOption", "skip_before_action :login, only: :show, if: :trusted?", AutoCorrect="disabled")
    for source in ("skip_before_action :login, :only => :show, :if => :trusted?", "skip_before_action :login, \"only\": :show, \"if\": :trusted?", "skip_before_action :login, only: :show,\n if: :trusted?", "skip_before_action :login, only: :show, if: -> { # keep\n trusted? }", "skip_before_action(:login, only: :show, if: :trusted?)", "skip_before_action :login, {only: :show, if: :trusted?,}", "skip_before_action :login, **options", "skip_before_action :login, if: :first, only: :show, if: :last"):
        add("Rails/IgnoredSkipActionFilterOption", source)
    for source in ("create_table :users", "create_table(:users)", "create_table", "create_table = :users; create_table", "def create_table; end", "create_table :users, id: false", "create_table :users, 'id' => false", "create_table :users, \"id\": false", "create_table :users, &:timestamps", "create_table 'users', &:timestamps", "other.create_table :users", "create_table :users do |t|; end", "create_table(:users) { |t| t.string :name }", "create_table :users do |t| t.timestamps; end", "create_table :users do |t| t.datetime :created_at; end", "create_table :users do |t| t.datetime 'updated_at'; end", "create_table :users do |t| other.timestamps; end", "create_table :users do |t| timestamps; end", "create_table :users do |t| t&.timestamps; end", "create_table :users do |t| if ready?; t.timestamps; end; end", "create_table :users do |t| t.timestamps(extra); end", "create_table :users do; end", "create_table :users do; timestamps; end", "create_table :users do |t, other| t.timestamps; end", "create_table :users do |t; other| t.timestamps; end", "create_table :users do |t| t.string :name, id: false; end", "create_table :users, {other: {id: false}}", "create_table(:users) { _1.timestamps }", "create_table :users do |t| timestamps = true; timestamps; end"):
        add("Rails/CreateTableWithTimestamps", source)
    for source in ("def has_and_belongs_to_many; end", "def self.has_and_belongs_to_many; end", "alias has_and_belongs_to_many existing", "undef has_and_belongs_to_many"):
        add("Rails/HasAndBelongsToMany", source)
    add("Rails/OutputSafety", "raw(def t; end)")
    for source in ("create_table :users do |t| def timestamps; end; end", "create_table :users do |t| t.timestamps = true; end", "create_table :users do |t| other.datetime = :created_at; end"):
        add("Rails/CreateTableWithTimestamps", source)
    for cop, source in (("Rails/ArelStar", "Model['*'] = value"), ("Rails/FindBy", "User.where(name: 'Ada').take = value"), ("Rails/Pick", "User.pluck(:id).first = value"), ("Rails/OutputSafety", "output.html_safe = value"), ("Performance/Caller", "caller.first = value"), ("Performance/Caller", "caller[0] = value"), ("Performance/Count", "items.select { |item| item }.count = value"), ("Performance/Size", "[1].count = value"), ("Performance/FixedSize", "[1].size = value"), ("Performance/FlatMap", "items.map { |item| [item] }.flatten = value"), ("Performance/UriDefaultParser", "URI::Parser.new = value"), ("Performance/ReverseEach", "items.reverse.each = value"), ("Performance/UnfreezeString", "String.new = value"), ("Performance/BindCall", "method.bind(object).call = value"), ("Performance/Size", "[1].count, other = values"), ("Performance/Size", "for [1].count in values; end"), ("Performance/Size", "[1].count += 1"), ("Performance/Size", "[1].count.foo = value")):
        add(cop, source)
    for source in ("raw(I18n&.t(:name))", "raw(I18n.t = input)", "raw(:: I18n.t(:name))"):
        add("Rails/OutputSafety", source)
    from expression_cases import extend
    extend(add)
    for source in ("Date.today", "::Date.today", "Other::Date.today", "Date&.today", "Date.current", "Date.yesterday", "Date.tomorrow", "Date.new.today", "Date.today.current", "Date.today.today", "wrapper(Date.today).tomorrow", "wrapper { Date.today }.current", "Date.today { work }", "Date.today = value", "date.to_time", "date.to_time(:utc)", "date&.to_time", "date.to_time_in_current_zone", "date.to_time_in_current_zone()", "date.to_time.utc", "date.to_time.in_time_zone", "wrapper(date.to_time).utc", "wrapper { date.to_time }.utc", "'2020-01-01T00:00:00Z'.to_time", "'2020-01-01+09:30'.to_time", "'2020-01-01Z'.to_time", "'2020-01-01'.to_time", "'2020-01-01'.to_time(:utc)"):
        add("Rails/Date",source)
        add("Rails/Date",source,AllowToTime=False)
        add("Rails/Date",source,EnforcedStyle="strict",AllowToTime=False)
    add("Rails/Date","Date.today",AutoCorrect="disabled")
    add("Rails/Date","date.to_time_in_current_zone",AutoCorrect="disabled")
    for method in ("in_time_zone", "utc", "getlocal", "xmlschema", "iso8601", "jisx0301", "rfc3339", "httpdate", "to_i", "to_f"):
        add("Rails/Date",f"date.to_time.{method}",AllowToTime=False)
        add("Rails/Date",f"date.to_time.{method}",EnforcedStyle="strict",AllowToTime=False)
    add("Rails/Date","date.to_time",EnforcedStyle="strict")
    add("Rails/Date","Date.today.to_time",EnforcedStyle="strict")
    add("Rails/Date", ":'2020-01-01Z'.to_time",AllowToTime=False)
    return rows
