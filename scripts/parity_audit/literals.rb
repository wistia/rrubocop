# frozen_string_literal: true

require 'rubocop'
require 'json'
require_relative 'literals/variants'
require_relative 'mutations/source'
require_relative 'mutations/arguments'
require_relative 'mutations/operators'
require_relative 'mutations/blocks'
require_relative 'mutations/numbers'
require_relative 'mutations/literals'
require_relative 'mutations/encodings'

module AuditLiterals
  def self.expand(test)
    parsed = RuboCop::ProcessedSource.new(test.fetch('source'), 3.4)
    return [] unless parsed.valid_syntax?

    legacy = candidates(parsed.ast).flat_map { |node| changes(test, node) }
    legacy + ast_changes(test, parsed)
  end

  def self.ast_changes(test, parsed)
    source = AuditMutationSource.canonical(parsed)
    generators.flat_map { |generator| generator.expand(source, parsed.ast) }.filter_map do |kind, changed|
      AuditMutationSource.row(test, kind, changed) if changed != source
    end
  end

  def self.generators
    [AuditArgumentMutations, AuditOperatorMutations, AuditBlockMutations,
     AuditNumberMutations, AuditNestedLiteralMutations, AuditEncodingMutations]
  end

  def self.candidates(ast)
    return [] unless ast

    ast.each_node.select { |node| (node.str_type? || node.sym_type?) && node.loc.expression }.first(2)
  end

  def self.changes(test, node)
    AuditLiteralVariants.forms(node).map do |kind, replacement|
      source = replace(test.fetch('source'), node.loc.expression, replacement)
      AuditMutationSource.row(test, kind, source)
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
