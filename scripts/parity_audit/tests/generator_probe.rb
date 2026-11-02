# frozen_string_literal: true

require 'rubocop'
require 'json'
require_relative '../mutations/source'

module GeneratorProbe
  def self.compile(source)
    RubyVM::InstructionSequence.compile(source)
    { 'valid' => true }
  rescue SyntaxError => e
    { 'valid' => false, 'error' => e.message }
  end

  def self.inspect_source(source)
    parsed = AuditMutationSource.parse(source)
    result = { 'compile' => compile(source), 'valid_parser' => parsed.valid_syntax? }
    return result unless parsed.valid_syntax? && parsed.ast

    result.merge('ast' => AuditMutationSource.key(parsed.ast).inspect,
                 'symbols' => symbols(parsed.ast), 'numbers' => numbers(parsed.ast))
  rescue StandardError => e
    { 'compile' => compile(source), 'parser_error' => "#{e.class}: #{e.message}" }
  end

  def self.symbols(ast)
    ast.each_node.filter_map do |node|
      next unless node.sym_type?

      value = node.children.first.to_s
      { 'encoding' => value.encoding.name, 'bytes' => value.bytes }
    end
  end

  def self.numbers(ast)
    ast.each_node.filter_map do |node|
      next unless node.int_type? || node.float_type?

      value = node.children.first
      { 'class' => value.class.name, 'value' => value.to_s,
        'bits' => value.is_a?(Float) ? [value].pack('G').unpack1('H*') : nil }
    end
  end
end

input = JSON.parse(File.read(ARGV.fetch(0)))
File.write(ARGV.fetch(1), JSON.generate(input.map { |source| GeneratorProbe.inspect_source(source) }))
