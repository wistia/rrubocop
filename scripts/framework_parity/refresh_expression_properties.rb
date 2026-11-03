# frozen_string_literal: true

require "rubocop"
require "json"

expressions = ["1", "- 1", "+1", "+ 1", "-1.0", "- 1.0", "1r", "1i", "nil", "true", "false", "self", "CONST", "provider::CONST", "@value", "@@value", "$value", "__LINE__", "__FILE__", "__ENCODING__", "value", "!value", "(not value)", "!1", "defined?(value)", "1 + 2", "1 < 2", "true && 1", "false || nil", "false && work", "[]", "{}", "[1, :a]", "{a: 1}", "{value:}", "{User:}", "{__LINE__:}", "[*value]", "{**value}", "(1..3)", "(1)", '"text"', ":text", ':""', "%s()", '"#{value}"', '"#{work()}"', "%w[a b]", "%i[a b]", '%W[a#{value} b]', '%I[a#{value} b]', '/#{value}/', "if true; 1; else; 2; end", "while false; nil; end", "case 1; when 1; 2; else; 3; end"]
sources = expressions.map { |expression| "probe(#{expression})\n" }
sources += ['value=1; probe(value)', 'value=1; probe(!value)', 'value=1; probe("#{value}")', 'value=1; probe(%W[a#{value}])', 'value=1; probe(%I[a#{value}])', 'value=1; probe({value:})', 'items.each { probe(_1) }', 'probe(_1)']
sources += ["probe(<<~TEXT)\nplain\nTEXT", "probe(<<~TEXT)\n\#{work}\nTEXT", "value=1; probe(<<~TEXT)\n\#{value}\nTEXT", "probe(<<~`TEXT`)\nnever_executed\nTEXT"]
rows = sources.map do |source|
  parsed = RuboCop::ProcessedSource.new(source, 3.4)
  raise "Invalid property source: #{source}" unless parsed.ast

  target = parsed.ast.each_node(:send).find { |node| node.method_name == :probe }.first_argument
  {source:, pure: target.pure?, ast_type: target.type}
end
contexts = ["target()", "target(); later", "earlier; target()", "def m; target(); end", "def m; target(); later; end", "items.each { target() }", "items.each { target(); later }", "result = target()", "return target()", "consume(target())", "[target()]", "result = [target()]", "{target() => 1}", "defined?(target())", "result=defined?(target())", "if target(); work; end", "if flag; target(); end", "value = if flag; target(); end", "while flag; target(); end", "while target(); work; end", "for value in values; target(); end", "value = for element in values; target(); end", "case value; when target(); work; end", "case target(); when value; work; end", "case value; when key; target(); end", "!target()", "target() && work", "(target())", "begin; target(); end", "begin; target(); ensure; work; end", "(target()..3)", '"#{target()}"', '/#{target()}/', "items.each_with_object({}) { |item,hash| target() }"]
rows += contexts.map do |source|
  parsed = RuboCop::ProcessedSource.new(source, 3.4)
  raise "Invalid property source: #{source}" unless parsed.ast

  target = parsed.ast.each_node(:send).find { |node| node.method_name == :target }
  {source:, value_used: target.value_used?, parent: target.parent&.type}
end
File.write(ARGV.fetch(0), JSON.pretty_generate(rows) + "\n")
puts JSON.generate(observations: rows.size)
