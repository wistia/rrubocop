# frozen_string_literal: true

require_relative 'source'

module AuditArgumentMutations
  COMMENT = "\n# parity audit\n"
  CALLS = %i[send csend super yield].freeze

  def self.expand(source, ast)
    return [] unless ast

    ast.each_node.flat_map { |node| argument_changes(source, ast, node) }
  end

  def self.arguments(node)
    return node.arguments if CALLS.include?(node.type)
    return [node.children.last] if %i[op_asgn or_asgn and_asgn].include?(node.type)

    []
  end

  def self.argument_changes(source, ast, node)
    values = arguments(node)
    return empty_comment(source, ast, node) if CALLS.include?(node.type) && values.empty?

    values.each_with_index.filter_map do |argument, index|
      next unless argument.is_a?(AST::Node) && argument.loc.expression

      changed = comment(source, ast, node, argument)
      ["ast-argument-comment:#{node.loc.expression.begin_pos}:#{index}", changed] if changed
    end
  end

  def self.empty_comment(source, ast, node)
    position = node.loc.respond_to?(:begin) && node.loc.begin&.end_pos
    text = position ? COMMENT : "(#{COMMENT})"
    changed = AuditMutationSource.insert(source, position || node.loc.expression.end_pos, text)
    return [] unless AuditMutationSource.same_ast?(changed, ast)

    [["ast-empty-arguments:#{node.loc.expression.begin_pos}", changed]]
  end

  def self.comment(source, ast, node, argument)
    changed = AuditMutationSource.insert(source, argument.loc.expression.begin_pos, COMMENT)
    return changed if AuditMutationSource.same_ast?(changed, ast)

    parenthesized(source, ast, node, argument)
  end

  def self.parenthesized(source, ast, node, argument)
    return unless (node.send_type? || node.csend_type?) && node.loc.selector && node.arguments.any?

    changed = wrap_arguments(source, node, argument)
    changed if AuditMutationSource.same_ast?(changed, ast)
  end

  def self.wrap_arguments(source, node, argument)
    first, last = argument_span(node)
    arguments = AuditMutationSource.insert(source[first...last], argument.loc.expression.begin_pos - first, COMMENT)
    AuditMutationSource.replace_range(source, node.loc.selector.end_pos, last, "(#{arguments})")
  end

  def self.argument_span(node)
    [node.arguments.first.loc.expression.begin_pos, node.arguments.last.loc.expression.end_pos]
  end
end
