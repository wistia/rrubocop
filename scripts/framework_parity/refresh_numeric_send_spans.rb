# frozen_string_literal: true

require 'rubocop'
require 'json'

abort 'Expected RuboCop 1.65.1' unless RuboCop::Version::STRING == '1.65.1'
rows = JSON.parse(File.read(ARGV.fetch(0)))
rows.each do |row|
  processed = RuboCop::ProcessedSource.new(row.fetch('source'), row.fetch('ruby_version'))
  abort "Invalid original source: #{row.fetch('source')}" unless processed.valid_syntax?

  calls = processed.ast.each_node(:send, :csend).filter_map do |node|
    next unless node.loc.dot

    selector = node.loc.selector
    expression = node.loc.expression
    { selector: [selector.line, selector.column], start: [expression.line, expression.column],
      end: [expression.last_line, expression.last_column], source: node.source }
  end
  row['calls'] = calls.sort_by { |record| record[:selector] }
end
File.write(ARGV.fetch(1), "#{JSON.pretty_generate(rows)}\n")
