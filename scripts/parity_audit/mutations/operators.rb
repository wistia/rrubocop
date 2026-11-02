# frozen_string_literal: true

require_relative 'source'
require_relative 'arguments'

module AuditOperatorMutations
  OPERATORS = %i[+ - * / % ** << >> == === != < > <= >= <=> & | ^ =~ !~ [] []= ! ~ +@ -@].freeze

  def self.expand(source, ast)
    return [] unless ast

    ast.each_node.flat_map { |node| variants(source, ast, node) }
  end

  def self.variants(source, ast, node)
    return [] unless node.send_type? && node.receiver && OPERATORS.include?(node.method_name)

    methods(node).filter_map do |method|
      replacement = explicit_call(source, node, method)
      changed = AuditMutationSource.replace(source, node.loc.expression, replacement)
      next unless AuditMutationSource.same_ast?(changed, ast, { node => method }.compare_by_identity)

      ["operator-call-comment:#{node.loc.expression.begin_pos}:#{method}", changed]
    end
  end

  def self.methods(node)
    node.method?(:==) ? %i[== eql? equal?] : [node.method_name]
  end

  def self.explicit_call(source, node, method)
    receiver = node.receiver.loc.expression.source
    receiver = "(#{receiver})" unless atomic?(node.receiver)
    arguments = node.arguments.map { |arg| source[arg.loc.expression.begin_pos...arg.loc.expression.end_pos] }
    "#{receiver}.#{method}(#{AuditArgumentMutations::COMMENT}#{arguments.join(', ')})"
  end

  def self.atomic?(node)
    return true if %i[const lvar ivar cvar gvar self str sym array hash].include?(node.type)
    return !node.loc.expression.source.start_with?('-', '+') if %i[int float].include?(node.type)

    (node.send_type? || node.csend_type?) && node.loc.respond_to?(:dot) && node.loc.dot
  end
end
