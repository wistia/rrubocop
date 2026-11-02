# frozen_string_literal: true

require_relative 'source'

module AuditBlockMutations
  BLOCK = ' { |__parity_value| __parity_value }'

  def self.expand(source, ast)
    return [] unless ast

    ast.each_node.filter_map { |node| change(source, node) }
  end

  def self.change(source, node)
    return unless call?(node)

    receiver = node.receiver
    return unless call?(receiver)
    return if receiver.arguments.any?(&:block_pass_type?)

    changed = AuditMutationSource.insert(source, receiver.loc.expression.end_pos, BLOCK)
    return unless attached?(changed, node, receiver)

    ["attached-receiver-block:#{receiver.loc.expression.begin_pos}", changed]
  end

  def self.attached?(source, original, receiver)
    parsed = AuditMutationSource.parse(source)
    return false unless parsed.valid_syntax?

    parsed.ast.each_node.any? { |node| matching_receiver?(node, original, receiver) }
  end

  def self.call?(node)
    node && (node.send_type? || node.csend_type?)
  end

  def self.same_start?(node, original)
    node.loc.expression.begin_pos == original.loc.expression.begin_pos
  end

  def self.matching_receiver?(node, original, receiver)
    return false unless call?(node) && same_start?(node, original)
    return false unless node.method_name == original.method_name && node.receiver&.block_type?

    AuditMutationSource.key(node.receiver.send_node) == AuditMutationSource.key(receiver)
  end
end
