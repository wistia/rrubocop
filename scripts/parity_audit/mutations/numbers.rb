# frozen_string_literal: true

require_relative 'source'

module AuditNumberMutations
  def self.expand(source, ast)
    return [] unless ast

    ast.each_node.flat_map do |node|
      forms(node).filter_map do |kind, replacement|
        changed = AuditMutationSource.replace(source, node.loc.expression, replacement)
        [kind, changed] if changed != source && AuditMutationSource.same_ast?(changed, ast)
      end
    end
  end

  def self.forms(node)
    return integers(node.children.first) if node.int_type?
    return floats(node.children.first) if node.float_type?

    []
  end

  def self.integers(value)
    sign = value.negative? ? '-' : ''
    forms = [['integer-hex', "#{sign}0x#{value.abs.to_s(16)}"],
             ['integer-binary', "#{sign}0b#{value.abs.to_s(2)}"],
             ['integer-octal', "#{sign}0o#{value.abs.to_s(8)}"]]
    digits = value.abs.to_s
    forms << ['integer-underscores', sign + digits.reverse.scan(/.{1,3}/).join('_').reverse] if digits.length > 3
    forms
  end

  def self.floats(value)
    return [] unless value.finite?

    mantissa, exponent = value.to_s.split('e', 2)
    exponent = exponent ? "e#{exponent}" : ''
    [['float-scientific', format('%.17e', value)],
     ['float-trailing-zero', "#{mantissa}0#{exponent}"],
     ['float-underscores', "#{mantissa}_0#{exponent}"]]
  end
end
