# frozen_string_literal: true

# Replays captured sources with fresh RuboCop instances to avoid retained ignored-node ranges.
require "bundler/setup"
require "rubocop"
require "json"
abort "Expected RuboCop 1.65.1" unless RuboCop::Version::STRING == "1.65.1"
input, output = ARGV
abort "Usage: refresh_else_layout_states.rb INPUT_JSON OUTPUT_JSON" unless input && output
module OriginalElseLayoutReference
  def self.without_grouping(node)
    return node unless node.is_a?(RuboCop::AST::Node)
    return without_grouping(node.children.first) if node.begin_type? && node.children.one?

    [node.type, *node.children.map { |child| without_grouping(child) }]
  end
end
cases = JSON.parse(File.read(input))
cases.each do |entry|
  entry["reference_version"] = RuboCop::Version::STRING
  entry["parser_version"] = Gem.loaded_specs.fetch("parser").version.to_s
  options = RuboCop::ConfigLoader.default_configuration.for_cop("Lint/ElseLayout").merge(entry.fetch("config"))
  options["Enabled"] = true
  config = RuboCop::Config.new({"AllCops" => {"TargetRubyVersion" => options.fetch("TargetRubyVersion")},
                                "Lint/ElseLayout" => options}.merge(entry.fetch("sibling_cops", {})))
  klass = RuboCop::Cop::Lint::ElseLayout
  entry["effective_indentation_width"] = options["IndentationWidth"] || config.for_cop("Layout/IndentationWidth")["Width"] || 2
  cop = klass.new(config, autocorrect: true)
  processed = RuboCop::ProcessedSource.new(entry.fetch("original"), options.fetch("TargetRubyVersion"))
  entry["ruby_parser_diagnostics"] = processed.diagnostics.map { |diagnostic| {level: diagnostic.level.to_s, reason: diagnostic.reason.to_s, line: diagnostic.location.line} }
  processed.config = config
  processed.registry = RuboCop::Cop::Registry.new([klass])
  report = RuboCop::Cop::Team.new([cop], config, raise_error: false).investigate(processed)
  if report.errors.any?
    entry["ruby_cop_errors"] = report.errors.map { |error| {class: error.cause.class.name, message: error.cause.message} }
    entry["ruby_cop_error"] = entry.fetch("ruby_cop_errors").first
    next
  end
  entry["ruby_diagnostics"] = report.offenses.reject(&:disabled?).sort.map do |offense|
    {line: offense.line, column: offense.column + 1, length: offense.location.length, message: offense.message, correctable: offense.correctable?, corrected: offense.corrected?}
  end
  corrector = report.correctors.compact.find { |candidate| !candidate.empty? }
  entry["ruby_corrected"] = corrector ? corrector.rewrite : processed.raw_source
  corrected = RuboCop::ProcessedSource.new(entry.fetch("ruby_corrected"), options.fetch("TargetRubyVersion"))
  entry["ruby_ast"] = processed.ast&.inspect
  entry["ruby_corrected_ast"] = corrected.ast&.inspect
  entry["ruby_corrected_parser_diagnostics"] = corrected.diagnostics.map { |diagnostic| {level: diagnostic.level.to_s, reason: diagnostic.reason.to_s} }
  entry["ruby_preserves_ast_without_grouping"] = OriginalElseLayoutReference.without_grouping(processed.ast) == OriginalElseLayoutReference.without_grouping(corrected.ast)
  entry.delete("ruby_safe")
  entry["reference_investigation"] = "fresh cop per independent source/config"
end
File.write(output, "#{JSON.pretty_generate(cases)}\n")
