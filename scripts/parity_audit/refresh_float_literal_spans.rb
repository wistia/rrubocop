# frozen_string_literal: true

require "bundler/setup"
require "rubocop"
require "json"

abort "Expected RuboCop 1.65.1" unless RuboCop::Version::STRING == "1.65.1"
rows = JSON.parse(File.read(ARGV.fetch(0)))
rows.each do |row|
  processed = RuboCop::ProcessedSource.new(row.fetch("source"), row.fetch("target_ruby_version"))
  abort "Invalid original source: #{row.fetch("source")}" unless processed.valid_syntax?

  row["parser_version"] = Gem.loaded_specs.fetch("parser").version.to_s
  row["floats"] = processed.ast ? processed.ast.each_node(:float).map do |node|
    {line: node.loc.expression.line, column: node.loc.expression.column + 1,
     length: node.loc.expression.length, source: node.source, value: node.children.first.to_s}
  end : []
  row["calls"] = processed.ast ? processed.ast.each_node(:send, :csend).filter_map do |node|
    next unless node.loc.dot

    {selector: [node.loc.selector.line, node.loc.selector.column],
     start: [node.loc.expression.line, node.loc.expression.column]}
  end : []
end
File.write(ARGV.fetch(1), "#{JSON.pretty_generate(rows)}\n")
