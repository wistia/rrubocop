# frozen_string_literal: true

module AuditLiteralVariants
  def self.forms(node)
    value = node.children.first.to_s
    return [] unless value.match?(%r{\A[a-zA-Z0-9_./:-]*\z})

    node.sym_type? ? symbols(value) : strings(value)
  end

  def self.encoded(value)
    escaped = value.bytes.map { |byte| format('\\x%02x', byte) }.join
    %("#{escaped}")
  end

  def self.strings(value)
    forms = [['percent-string', "%q|#{value}|"], ['encoded-string', encoded(value)],
             ['opposite-quote-string', "#{value}'".dump]]
    forms << ['character-string', "?#{value}"] if value.length == 1
    forms
  end

  def self.symbols(value)
    [['percent-symbol', "%s|#{value}|"], ['encoded-symbol', ":#{encoded(value)}"],
     ['opposite-quote-symbol', ":#{"#{value}'".dump}"]]
  end
end
