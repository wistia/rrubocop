# frozen_string_literal: true

# Replays captured sources with fresh RuboCop instances to avoid retained ignored-node ranges.
require "bundler/setup"
require "rubocop"
require "json"
abort "Expected RuboCop 1.65.1" unless RuboCop::Version::STRING == "1.65.1"
input, output = ARGV
abort "Usage: refresh_security_eval_states.rb INPUT_JSON OUTPUT_JSON" unless input && output
cases = JSON.parse(File.read(input))
cases.each do |entry|
  entry["reference_version"] = RuboCop::Version::STRING
  entry["parser_version"] = Gem.loaded_specs.fetch("parser").version.to_s
  options = RuboCop::ConfigLoader.default_configuration.for_cop("Security/Eval").merge(entry.fetch("config"))
  options["Enabled"] = true
  config = RuboCop::Config.new("AllCops" => {"TargetRubyVersion" => options.fetch("TargetRubyVersion")},
    "Security/Eval" => options)
  klass = RuboCop::Cop::Security::Eval
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
    {line: offense.line, column: offense.column + 1, length: offense.location.length, message: offense.message, severity: offense.severity.name.to_s, correctable: offense.correctable?, corrected: offense.corrected?}
  end
  corrector = report.correctors.compact.find { |candidate| !candidate.empty? }
  entry["ruby_corrected"] = corrector ? corrector.rewrite : processed.raw_source
  entry["ruby_ast"] = processed.ast&.inspect
  entry["ruby_eval_shapes"] = if processed.ast
    processed.ast.each_node(:send).select { |node| node.method_name == :eval }.map do |node|
      {receiver_type: node.receiver&.type, first_argument_type: node.first_argument&.type,
       first_dstr_recursive_literal: node.first_argument&.dstr_type? ? node.first_argument.recursive_literal? : nil}
    end
  else
    []
  end
  entry.delete("ruby_safe")
  entry["reference_investigation"] = "fresh cop per independent source/config"
end
File.write(output, "#{JSON.pretty_generate(cases)}\n")
