# frozen_string_literal: true

require_relative 'source'
require_relative '../literals/variants'

module AuditNestedLiteralMutations
  def self.expand(source, ast)
    return [] unless ast

    ast.each_node.flat_map { |node| variants(source, ast, node) }
  end

  def self.variants(source, ast, node)
    return [] unless node.str_type? || node.sym_type?
    return [] if node.loc.respond_to?(:heredoc_body)

    forms(node).filter_map do |kind, replacement|
      changed = replace(source, node, replacement)
      [kind, changed] if valid?(changed, ast, kind)
    end
  end

  def self.forms(node)
    value = node.children.first.to_s
    equivalent = [['nested-encoded-literal', AuditLiteralVariants.encoded(value)]]
    equivalent[0][1] = ":#{equivalent[0][1]}" if node.sym_type?
    AuditLiteralVariants.forms(node) + equivalent
  end

  def self.replace(source, node, replacement)
    return AuditMutationSource.replace(source, node.loc.expression, replacement) unless label_key?(node)

    AuditMutationSource.replace_range(source, node.loc.expression.begin_pos,
                                      node.parent.loc.operator.end_pos, label_replacement(node, replacement))
  end

  def self.label_replacement(node, replacement)
    finish = node.parent.loc.operator.end_pos
    value = node.parent.value.loc.expression
    "#{replacement} =>#{value.begin_pos < finish ? " #{value.source}" : ''}"
  end

  def self.label_key?(node)
    node.parent&.pair_type? && node.parent.key.equal?(node) && node.parent.loc.operator&.source == ':'
  end

  def self.valid?(source, ast, kind)
    return AuditMutationSource.parse(source).valid_syntax? if kind.start_with?('opposite-quote')

    AuditMutationSource.same_ast?(source, ast)
  end
end
