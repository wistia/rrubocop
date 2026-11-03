# frozen_string_literal: true

require "bundler/setup"
require "rubocop"
require "json"
cases = JSON.parse(File.read(ARGV.fetch(0)))
cases.each do |entry|
  source = RuboCop::ProcessedSource.new(entry.fetch("original"), 3.4)
  entry["reference_version"] = RuboCop::Version::STRING
  entry["parser_version"] = Parser::VERSION
  entry["ast_version"] = Gem.loaded_specs.fetch("rubocop-ast").version.to_s
  entry["runtime_ruby_version"] = RUBY_VERSION
  entry["target_ruby_version"] = 3.4
  entry["parser_errors"] = source.diagnostics.select { |d| [:error, :fatal].include?(d.level) }.map(&:render)
  entry["ruby_roles"] = (source.ast ? source.ast.each_node(:pair).select(&:value_omission?) : []).map do |pair|
    value = pair.value
    {byte: source.buffer.source[0...value.source_range.begin_pos].bytesize, name: pair.key.value.to_s, node_type: value.type, local: value.lvar_type?, pure: value.pure?}
  end
end
File.write(ARGV.fetch(1), JSON.pretty_generate(cases) + "\n")
