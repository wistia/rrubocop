# frozen_string_literal: true

require "json"
require "prism"

def literal_parts(node)
  case node
  when Prism::StringNode
    node.unescaped
  when Prism::InterpolatedStringNode
    node.parts.map { |part| literal_parts(part) }.join
  else
    "HOST_INTERPOLATION_MARKER"
  end
end

upstream, inventory_path, output, manifest = ARGV
inventory = JSON.parse(File.read(inventory_path)).index_by { |row| row.fetch("name") }
cases = []
not_imported = []
Dir[File.join(upstream, "*_spec.rb")].sort.each do |path|
  pending = [Prism.parse_file(path).value]
  calls = []
  until pending.empty?
    node = pending.pop
    calls << node if node.is_a?(Prism::CallNode) && %i[expect_offense expect_no_offenses].include?(node.name)
    pending.concat(node.compact_child_nodes)
  end
  calls.sort_by { |call| call.location.start_offset }.each_with_index do |call, index|
    id = "#{File.basename(path, ".rb")}:#{index}"
    argument = call.arguments&.arguments&.first
    unless argument.is_a?(Prism::StringNode) || argument.is_a?(Prism::InterpolatedStringNode)
      not_imported << {id:, reason: "Fixture template requires host interpolation"}
      next
    end
    source = literal_parts(argument).lines.reject { |line| line.match?(/\A\s*[\^~]+/) }.join
    if source.include?("HOST_INTERPOLATION_MARKER") || source.match?(/%\{\w+\}/)
      not_imported << {id:, reason: "Fixture template requires parameter substitution"}
      next
    end
    name = File.basename(path, "_spec.rb").split("_").map(&:capitalize).join
    cop = "RSpec/#{name}"
    options = inventory.fetch(cop).fetch("scopes").fetch("Gemfile").merge("__RubocopVersion" => "1.65.1", "TargetRubyVersion" => 3.4)
    cases << {id:, cop:, source:, path: "/tmp/spec/parity_spec.rb", config: options, expected: [], corrected: source, origin: "reference", upstream: "rubocop-rspec@3.0.4/spec/rubocop/cop/rspec/#{File.basename(path)}##{index}"}
  end
end
File.write(output, JSON.pretty_generate(cases) + "\n")
File.write(manifest, JSON.pretty_generate({upstream_version: "rubocop-rspec 3.0.4", imported: cases.length, not_imported:}) + "\n")
puts "Imported #{cases.length} original Ruby specimens; #{not_imported.length} host-generated templates remain explicit gaps"
