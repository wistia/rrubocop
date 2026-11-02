# frozen_string_literal: true

require 'rubocop'
require 'json'
require_relative 'literals/variants'

module AuditLiterals
  def self.expand(test)
    parsed = RuboCop::ProcessedSource.new(test.fetch('source'), 3.4)
    return [] unless parsed.valid_syntax?

    candidates(parsed.ast).flat_map { |node| changes(test, node) }
  end

  def self.candidates(ast)
    return [] unless ast

    ast.each_node.select { |node| (node.str_type? || node.sym_type?) && node.loc.expression }.first(2)
  end

  def self.changes(test, node)
    AuditLiteralVariants.forms(node).map do |kind, replacement|
      source = replace(test.fetch('source'), node.loc.expression, replacement)
      test.slice('cop', 'path', 'config').merge(
        'source' => source, 'expected' => [], 'corrected' => source, 'origin' => kind
      )
    end
  end

  def self.replace(source, location, replacement)
    source[0...location.begin_pos] + replacement + source[location.end_pos..]
  end
end

if $PROGRAM_NAME == __FILE__
  cases = JSON.parse(File.read(ARGV.fetch(0)))
  File.write(ARGV.fetch(1), JSON.generate(cases.flat_map { |test| AuditLiterals.expand(test) }))
end
