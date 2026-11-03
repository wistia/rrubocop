# frozen_string_literal: true

# Replays captured sources with fresh RuboCop instances to avoid retained ignored-node ranges.
require "bundler/setup"
require "rubocop"
require "json"
abort "Expected RuboCop 1.65.1" unless RuboCop::Version::STRING == "1.65.1"
input, output = ARGV
abort "Usage: refresh_interpolation_check_states.rb INPUT_JSON OUTPUT_JSON" unless input && output
module OriginalInterpolationReference
  def self.ast_types(node)
    node ? [node.type.to_s] + node.child_nodes.flat_map { |child| ast_types(child) } : []
  end
end
cases = JSON.parse(File.read(input))
cases.each do |entry|
  entry["reference_version"] = RuboCop::Version::STRING
  entry["parser_version"] = Gem.loaded_specs.fetch("parser").version.to_s
  options = RuboCop::ConfigLoader.default_configuration.for_cop("Lint/InterpolationCheck").merge(entry.fetch("config"))
  options["Enabled"] = true
  config = RuboCop::Config.new("AllCops" => {"TargetRubyVersion" => options.fetch("TargetRubyVersion")},
    "Lint/InterpolationCheck" => options)
  klass = RuboCop::Cop::Lint::InterpolationCheck
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
    {line: offense.line, column: offense.column + 1, length: offense.location.length, message: offense.message}
  end
  corrector = report.correctors.compact.find { |candidate| !candidate.empty? }
  entry["ruby_corrected"] = corrector ? corrector.rewrite : processed.raw_source
  corrected = RuboCop::ProcessedSource.new(entry.fetch("ruby_corrected"), options.fetch("TargetRubyVersion"))
  entry["ruby_ast_types"] = OriginalInterpolationReference.ast_types(processed.ast)
  entry["ruby_corrected_ast_types"] = OriginalInterpolationReference.ast_types(corrected.ast)
  entry["ruby_corrected_parser_diagnostics"] = corrected.diagnostics.map { |diagnostic| {level: diagnostic.level.to_s, reason: diagnostic.reason.to_s} }
  if entry.fetch("ruby_corrected") != entry.fetch("original")
    entry["rust_corrected"] = entry.fetch("original")
    entry["rust_correctable"] = false
    entry["reason"] = "Preserve literal interpolation expressions: the original delimiter rewrite activates evaluation or creates invalid syntax; retain the offense without an edit."
  end
  entry.delete("ruby_safe")
  entry["reference_investigation"] = "fresh cop per independent source/config"
end
File.write(output, "#{JSON.pretty_generate(cases)}\n")
