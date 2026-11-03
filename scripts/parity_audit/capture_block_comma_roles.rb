# frozen_string_literal: true

require "bundler/setup"
require "rubocop"
require "json"
abort "Expected RuboCop 1.65.1" unless RuboCop::Version::STRING == "1.65.1"
rows = JSON.parse(File.read(ARGV.fetch(0)))
cop = RuboCop::Cop::Style::TrailingCommaInBlockArgs.new
rows.each do |row|
  processed = RuboCop::ProcessedSource.new(row.fetch("original"), row.fetch("config", {}).fetch("TargetRubyVersion", 3.4))
  row["reference_version"] = RuboCop::Version::STRING
  row["parser_version"] = Gem.loaded_specs.fetch("parser").version.to_s
  row["ruby_parser_diagnostics"] = processed.diagnostics.map { |diagnostic| {level: diagnostic.level.to_s, reason: diagnostic.reason.to_s} }
  row["blocks"] = processed.ast&.each_node(:block)&.map do |node|
    count = cop.send(:arg_count, node)
    tokens = processed.tokens_within(node)
    pipes = tokens.select { |token| token.type == :tPIPE }.first(2)
    between = (pipes.size == 2) ? tokens[(tokens.index(pipes[0]) + 1)...tokens.index(pipes[1])] : []
    {source: node.source, line: node.loc.line, column: node.loc.column,
     parameter_source: node.arguments.source, parameter_types: node.arguments.children.map { |arg| arg.type.to_s },
     descendant_argument_count: count, lambda_literal: node.send_node.lambda_literal?,
     first_pipes: pipes.map { |token| [token.pos.line, token.pos.column] },
     last_token_between_pipes: between.last&.yield_self { |token| {type: token.type.to_s, source: token.text, line: token.pos.line, column: token.pos.column} },
     tokens: tokens.map { |token| {type: token.type.to_s, source: token.text, line: token.pos.line, column: token.pos.column} }}
  end || []
end
File.write(ARGV.fetch(1), "#{JSON.pretty_generate(rows)}\n")
