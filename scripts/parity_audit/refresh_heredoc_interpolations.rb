# frozen_string_literal: true

require 'bundler/setup'
require 'rubocop'
require 'json'

abort 'Expected RuboCop 1.65.1' unless RuboCop::Version::STRING == '1.65.1'
input, output = ARGV
abort 'Usage: refresh_heredoc_interpolations.rb INPUT_JSON OUTPUT_JSON' unless input && output

cases = JSON.parse(File.read(input))
cases.each do |entry|
  processed = RuboCop::ProcessedSource.new(entry.fetch('source'), entry.fetch('ruby_version'))
  invalid = processed.diagnostics.any? { |diagnostic| %i[error fatal].include?(diagnostic.level) }
  abort "Invalid original input: #{entry.fetch('id')}" if invalid

  entry['reference_version'] = RuboCop::Version::STRING
  heredocs = processed.ast.each_node.select { |node| node.loc.is_a?(Parser::Source::Map::Heredoc) }
  entry['heredocs'] = heredocs.map do |node|
    interpolations = node.child_nodes.reject(&:str_type?).map do |interpolation|
      expressions = interpolation.begin_type? ? interpolation.child_nodes : [interpolation]
      expressions.map do |expression|
        { source: expression.source, line: expression.first_line, column: expression.loc.expression.column }
      end
    end
    { opener: node.source, line: node.first_line, column: node.loc.expression.column,
      interpolations: }
  end
end
File.write(output, "#{JSON.pretty_generate(cases)}\n")
