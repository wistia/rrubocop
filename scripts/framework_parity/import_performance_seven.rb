# frozen_string_literal: true

require "json"
require "prism"

UNKNOWN = Object.new.freeze

def scalar(node, environment)
  case node
  when Prism::StringNode then node.unescaped
  when Prism::SymbolNode then node.unescaped.to_sym
  when Prism::IntegerNode, Prism::FloatNode then node.value
  when Prism::TrueNode then true
  when Prism::FalseNode then false
  when Prism::NilNode then nil
  when Prism::ArrayNode
    values = node.elements.map { |value| scalar(value, environment) }
    values.include?(UNKNOWN) ? UNKNOWN : values
  when Prism::HashNode, Prism::KeywordHashNode
    pairs = node.elements.map do |pair|
      next UNKNOWN unless pair.is_a?(Prism::AssocNode)

      key = scalar(pair.key, environment)
      value = scalar(pair.value, environment)
      [key.to_s, value] unless key.equal?(UNKNOWN) || value.equal?(UNKNOWN)
    end
    pairs.any? { |pair| pair.nil? || pair.equal?(UNKNOWN) } ? UNKNOWN : pairs.to_h
  when Prism::LocalVariableReadNode
    environment.fetch(node.name, UNKNOWN)
  when Prism::CallNode
    node.receiver.nil? && node.arguments.nil? ? environment.fetch(node.name, UNKNOWN) : UNKNOWN
  else UNKNOWN
  end
end

def literal_text(node, environment)
  return node.unescaped if node.is_a?(Prism::StringNode)
  return scalar(node, environment) if node.is_a?(Prism::LocalVariableReadNode) || node.is_a?(Prism::CallNode)
  return unless node.is_a?(Prism::InterpolatedStringNode)

  parts = node.parts.map do |part|
    if part.is_a?(Prism::EmbeddedStatementsNode)
      statements = part.statements&.body || []
      next UNKNOWN unless statements.one?

      scalar(statements.first, environment)
    else
      literal_text(part, environment)
    end
  end
  parts.map(&:to_s).join unless parts.any? { |part| part.nil? || part.equal?(UNKNOWN) }
end

def let_definition(node)
  return unless node.is_a?(Prism::CallNode) && node.name == :let

  name = node.arguments&.arguments&.first
  statements = node.block&.body&.body || []
  return unless name.is_a?(Prism::SymbolNode) && statements.one?

  [name.unescaped.to_sym, statements.first]
end

def visit(node, environment, config_expression, path, rows, unresolved)
  if node.is_a?(Prism::BlockNode)
    environment = environment.dup
    definitions = (node.body&.body || []).filter_map { |child| let_definition(child) }
    2.times do
      definitions.each do |name, expression|
        if name == :cop_config
          config_expression = expression
        else
          environment[name] = scalar(expression, environment)
        end
      end
    end
  end
  if node.is_a?(Prism::LocalVariableWriteNode)
    value = literal_text(node.value, environment)
    environment[node.name] = value.is_a?(String) ? value : scalar(node.value, environment)
  end
  if node.is_a?(Prism::CallNode) && %i[expect_offense expect_no_offenses autocorrect_source].include?(node.name)
    argument = node.arguments&.arguments&.first
    source = literal_text(argument, environment)
    config = config_expression ? scalar(config_expression, environment) : {}
    if source.is_a?(String) && config.is_a?(Hash)
      source = source.lines.reject { |line| line.match?(/\A[ \t]*\^/) }.join
      name = File.basename(path, "_spec.rb").split("_").map(&:capitalize).join
      rows << {cop: "Performance/#{name}", source:, config:, upstream: File.basename(path),
               line: node.location.start_line, assertion: node.name}
    else
      unresolved << {upstream: File.basename(path), line: node.location.start_line,
                     assertion: node.name, argument: argument&.class&.name, config_resolved: config.is_a?(Hash)}
    end
  end
  if node.is_a?(Prism::CallNode) && node.name == :each && node.block
    values = scalar(node.receiver, environment)
    parameters = node.block.parameters&.parameters&.requireds || []
    if values.is_a?(Array) && parameters.all? { |parameter| parameter.is_a?(Prism::RequiredParameterNode) }
      values.each do |value|
        child_environment = environment.dup
        parameters.zip(parameters.one? ? [value] : Array(value)).each do |parameter, binding|
          child_environment[parameter.name] = binding
        end
        visit(node.block, child_environment, config_expression, path, rows, unresolved)
      end
      return
    end
  end
  node.compact_child_nodes.each { |child| visit(child, environment, config_expression, path, rows, unresolved) }
end

input, output = ARGV
rows = []
unresolved = []
Dir[File.join(input, "*_spec.rb")].sort.each do |path|
  visit(Prism.parse_file(path).value, {}, nil, path, rows, unresolved)
end
File.write(output, JSON.pretty_generate(rows) + "\n")
File.write(output.sub(/\.json\z/, "-unresolved.json"), JSON.pretty_generate(unresolved) + "\n")
puts JSON.generate(imported: rows.size, unresolved: unresolved.size)
