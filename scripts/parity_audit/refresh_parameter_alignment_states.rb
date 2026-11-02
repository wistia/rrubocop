# frozen_string_literal: true

# Replays captured sources with fresh RuboCop instances to avoid retained ignored-node ranges.
require "bundler/setup"
require "rubocop"
require "json"
input, output = ARGV
abort "Usage: refresh_parameter_alignment_states.rb INPUT_JSON OUTPUT_JSON" unless input && output
cases = JSON.parse(File.read(input))
cases.each do |entry|
  options = entry.fetch("config")
  config = RuboCop::Config.new("AllCops" => {"TargetRubyVersion" => options.fetch("TargetRubyVersion")},
    "Layout/ParameterAlignment" => options, "Layout/IndentationWidth" => entry.fetch("sibling_cops", {}).fetch("Layout/IndentationWidth", {"Width" => options.fetch("IndentationWidth", 2)}))
  klass = RuboCop::Cop::Layout::ParameterAlignment
  cop = klass.new(config, autocorrect: true)
  processed = RuboCop::ProcessedSource.new(entry.fetch("original"), options.fetch("TargetRubyVersion"))
  entry["ruby_parser_diagnostics"] = processed.diagnostics.map { |diagnostic| {level: diagnostic.level.to_s, reason: diagnostic.reason.to_s, line: diagnostic.location.line} }
  processed.config = config
  processed.registry = RuboCop::Cop::Registry.new([klass])
  report = RuboCop::Cop::Team.new([cop], config, raise_error: true).investigate(processed)
  entry["ruby_diagnostics"] = report.offenses.reject(&:disabled?).sort.map do |offense|
    {line: offense.line, column: offense.column + 1, length: offense.location.length, message: offense.message}
  end
  corrector = report.correctors.compact.find { |candidate| !candidate.empty? }
  entry["ruby_corrected"] = corrector ? corrector.rewrite : processed.raw_source
  entry.delete("ruby_safe")
  entry["reference_investigation"] = "fresh cop per independent source/config"
end
File.write(output, "#{JSON.pretty_generate(cases)}\n")
