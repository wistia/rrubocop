# frozen_string_literal: true

module AuditMutationSource
  def self.parse(source)
    RuboCop::ProcessedSource.new(source, 3.4)
  end

  def self.canonical(parsed)
    parsed.buffer.source.dup
  end

  def self.replace(source, location, replacement)
    replace_range(source, location.begin_pos, location.end_pos, replacement)
  end

  def self.insert(source, position, text)
    replace_range(source, position, position, text)
  end

  def self.replace_range(source, first, last, replacement)
    source[0...first] + replacement.dup.force_encoding(source.encoding) + source[last..]
  end

  def self.row(test, kind, source)
    source = source.dup.force_encoding(Encoding::UTF_8)
    return unless source.valid_encoding?

    test.slice('cop', 'path', 'config').merge(
      'source' => source, 'expected' => [], 'corrected' => source, 'origin' => kind
    )
  end

  def self.same_ast?(source, original, methods = {})
    parsed = parse(source)
    parsed.valid_syntax? && key(parsed.ast) == key(original, methods)
  end

  def self.key(value, methods = {})
    return scalar_key(value) unless value.is_a?(AST::Node)
    return key(value.children.first, methods) if value.type == :begin && value.children.one?

    [value.type, children_key(value, methods)]
  end

  def self.children_key(value, methods)
    children = value.children.map { |child| key(child, methods) }
    method = methods[value]
    children[1] = scalar_key(method) if method
    children
  end

  def self.scalar_key(value)
    case value
    when String, Symbol then [value.class.name, value.to_s.encoding.name, value.to_s.bytes]
    when Float then [Float.name, [value].pack('G')]
    else value
    end
  end
end
