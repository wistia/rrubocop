# frozen_string_literal: true

require "json"
require "prism"

module RSpecUpstreamSources
  def self.literal_parts(node)
    case node
    when Prism::StringNode
      node.unescaped
    when Prism::InterpolatedStringNode
      node.parts.map { |part| literal_parts(part) }.join
    when Prism::EmbeddedStatementsNode
      expression = node.statements&.body&.first
      parameter = expression.is_a?(Prism::LocalVariableReadNode) && %i[include_method method].include?(expression.name)
      bare_call = expression.is_a?(Prism::CallNode) && expression.receiver.nil? && %i[include_method method].include?(expression.name) && expression.arguments.nil?
      if node.statements&.body&.one? && (parameter || bare_call)
        "%{#{expression.name}}"
      else
        "HOST_INTERPOLATION_MARKER"
      end
    else
      "HOST_INTERPOLATION_MARKER"
    end
  end

  def self.literal_filename(syntax, call, argument)
    return argument.unescaped if argument.is_a?(Prism::StringNode)
    return unless argument.is_a?(Prism::LocalVariableReadNode)

    blocks = []
    pending = [syntax]
    until pending.empty?
      node = pending.pop
      next unless node.location.start_offset <= call.location.start_offset && node.location.end_offset >= call.location.end_offset

      blocks << node if node.is_a?(Prism::BlockNode)
      pending.concat(node.compact_child_nodes)
    end
    block = blocks.min_by { |node| node.location.length }
    return unless block

    assignments = []
    pending = block.compact_child_nodes
    until pending.empty?
      node = pending.pop
      if node.is_a?(Prism::LocalVariableWriteNode) && node.name == argument.name && node.value.is_a?(Prism::StringNode) && node.location.start_offset < call.location.start_offset
        assignments << node
      end
      pending.concat(node.compact_child_nodes)
    end
    assignments.max_by { |node| node.location.start_offset }&.value&.unescaped
  end
end

upstream, inventory_path, output, manifest = ARGV
inventory = {}
JSON.parse(File.read(inventory_path)).each { |row| inventory.store(row.fetch("name"), row) }
cases = []
not_imported = []
Dir[File.join(upstream, "*_spec.rb")].sort.each do |path|
  syntax = Prism.parse_file(path).value
  pending = [syntax]
  calls = []
  until pending.empty?
    node = pending.pop
    calls << node if node.is_a?(Prism::CallNode) && %i[expect_offense expect_no_offenses expect_global_offense expect_no_global_offenses].include?(node.name)
    pending.concat(node.compact_child_nodes)
  end
  calls.sort_by { |call| call.location.start_offset }.each_with_index do |call, index|
    id = "#{File.basename(path, ".rb")}:#{index}"
    argument = call.arguments&.arguments&.first
    unless argument.is_a?(Prism::StringNode) || argument.is_a?(Prism::InterpolatedStringNode)
      not_imported << {id:, reason: "Fixture template requires host interpolation"}
      next
    end
    fixture_path = "/tmp/spec/parity_spec.rb"
    filename = call.arguments&.arguments&.[](1)
    filename = RSpecUpstreamSources.literal_filename(syntax, call, filename)
    if filename
      fixture_path = if filename.start_with?("/")
        filename
      elsif filename.start_with?("spec/")
        "/tmp/#{filename}"
      else
        "/tmp/spec/#{filename}"
      end
    elsif %i[expect_global_offense expect_no_global_offenses].include?(call.name)
      not_imported << {id:, reason: "Global offense filename requires host interpolation"}
      next
    end
    source = RSpecUpstreamSources.literal_parts(argument).lines.reject { |line| line.match?(/\A\s*[\^~]+/) }.join
    variants = if File.basename(path) == "repeated_include_example_spec.rb" && source.include?("%{include_method}")
      %w[include_examples it_behaves_like it_should_behave_like].map { |method| ["#{id}:#{method}", source.gsub("%{include_method}", method)] }
    elsif File.basename(path) == "subject_stub_spec.rb" && source.include?("%{method}")
      %w[describe xdescribe fdescribe context xcontext fcontext feature xfeature ffeature example_group shared_examples shared_examples_for shared_context].map { |method| ["#{id}:#{method}", source.gsub("%{method}", method)] }
    else
      [[id, source]]
    end
    variants.each do |variant_id, variant_source|
      if variant_source.include?("HOST_INTERPOLATION_MARKER") || variant_source.match?(/%\{\w+\}/)
        not_imported << {id: variant_id, reason: "Fixture template requires parameter substitution"}
        next
      end
      name = File.basename(path, "_spec.rb").split("_").map(&:capitalize).join
      cop = "RSpec/#{name}"
      options = inventory.fetch(cop).fetch("scopes").fetch("Gemfile").merge("__RubocopVersion" => "1.65.1", "TargetRubyVersion" => 3.4)
      cases << {id: variant_id, cop:, source: variant_source, path: fixture_path, config: options, expected: [], corrected: variant_source, origin: "reference", upstream: "rubocop-rspec@3.0.4/spec/rubocop/cop/rspec/#{File.basename(path)}##{index}"}
    end
  end
end
File.write(output, JSON.pretty_generate(cases) + "\n")
File.write(manifest, JSON.pretty_generate({upstream_version: "rubocop-rspec 3.0.4", imported: cases.length, not_imported:}) + "\n")
puts "Imported #{cases.length} original Ruby specimens; #{not_imported.length} host-generated templates remain explicit gaps"
