# frozen_string_literal: true

require "json"
require "prism"

def literal_text(node)
  return node.unescaped if node.is_a?(Prism::StringNode)
  return unless node.is_a?(Prism::InterpolatedStringNode)

  parts = node.parts.map { |part| literal_text(part) }
  parts.join if parts.none?(&:nil?)
end

input, output = ARGV
rows = []
unresolved = []
Dir[File.join(input, "*_spec.rb")].sort.each do |path|
  name = File.basename(path, "_spec.rb").split("_").map(&:capitalize).join
  pending = [Prism.parse_file(path).value]
  until pending.empty?
    node = pending.pop
    if node.is_a?(Prism::CallNode) && %i[expect_offense expect_no_offenses].include?(node.name)
      argument = node.arguments&.arguments&.first
      if (literal = literal_text(argument))
        source = literal.lines.reject { |line| line.match?(/\A[ \t]*\^/) }.join
        rows << {cop: "Performance/#{name}", source:, config: {}, upstream: File.basename(path), line: node.location.start_line}
      else
        unresolved << {upstream: File.basename(path), line: node.location.start_line, argument: argument&.class&.name}
      end
    end
    pending.concat(node.compact_child_nodes.reverse)
  end
end
File.write(output, JSON.pretty_generate(rows) + "\n")
File.write(output.sub(/\.json\z/, "-unresolved.json"), JSON.pretty_generate(unresolved) + "\n")
puts JSON.generate(imported: rows.size, unresolved: unresolved.size)
