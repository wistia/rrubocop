# frozen_string_literal: true

require "bundler/setup"
require "rubocop"
require "json"
rows = JSON.parse(File.read(ARGV.fetch(0)))
cops = ["with_first_argument", "with_fixed_indentation"].to_h do |style|
  config = RuboCop::Config.new("Layout/ArgumentAlignment" => {"EnforcedStyle" => style})
  [style, RuboCop::Cop::Layout::ArgumentAlignment.new(config)]
end
rows.each do |row|
  row["reference_version"] = RuboCop::Version::STRING
  row["parser_version"] = Gem.loaded_specs.fetch("parser").version.to_s
  row["ruby_version"] = row.fetch("ruby_version", 3.4)
  processed = RuboCop::ProcessedSource.new(row.fetch("source"), row.fetch("ruby_version", 3.4))
  row["parser_diagnostics"] = processed.diagnostics.map { |d| {level: d.level.to_s, reason: d.reason.to_s} }
  row["ast"] = processed.ast.inspect
  row["calls"] = processed.ast&.each_node(:send, :csend)&.map do |node|
    eligible = cops.values.first.send(:multiple_arguments?, node) && !(node.send_type? && node.method?(:[]=))
    {target: (node.loc.selector || (node.loc.respond_to?(:begin) ? node.loc.begin : nil) || node.loc.expression).yield_self { |r| [r.line, r.column] }, type: node.type.to_s, method: node.method_name.to_s, source: node.source, selector: node.loc.selector&.source, position: node.loc.selector&.yield_self { |r| [r.line, r.column] }, parent: node.parent&.type.to_s, arguments: node.arguments.map { |arg| {type: arg.type.to_s, source: arg.source, position: [arg.loc.line, arg.loc.column]} }, eligible: eligible, flattened: eligible ? cops.transform_values { |cop| cop.send(:flattened_arguments, node).map { |arg| {type: arg.type.to_s, source: arg.source, position: [arg.loc.line, arg.loc.column]} } } : {}}
  end || []
end
File.write(ARGV.fetch(1), JSON.pretty_generate(rows) + "\n")
