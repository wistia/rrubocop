# frozen_string_literal: true

require "bundler/setup"
require "rubocop"
require "json"
klass = RuboCop::Cop::Style::TernaryParentheses
samples = [
  "(items.any? { |item| item.active? }) ? a : b",
  "(items&.any? { |item| item.active? }) ? a : b",
  "(items.any? { _1.active? }) ? a : b",
  "items.any? { |item| item.active? } ? a : b",
  "(items.any?) ? a : b",
  "items.any? ? a : b",
  "(items.any?(value)) ? a : b",
  "(items.any? do |item| item.active? end) ? a : b"
]
rows = samples.flat_map do |sample|
  %w[require_no_parentheses require_parentheses require_parentheses_when_complex].filter_map do |style|
    version = 3.4
    text = sample + "\n"
    options = RuboCop::ConfigLoader.default_configuration.for_cop(klass.cop_name).merge("Enabled" => true, "TargetRubyVersion" => version, "EnforcedStyle" => style)
    config = RuboCop::Config.new("AllCops" => {"TargetRubyVersion" => version}, klass.cop_name => options)
    source = RuboCop::ProcessedSource.new(text, version)
    next unless source.valid_syntax?
    source.config = config
    source.registry = RuboCop::Cop::Registry.new([klass])
    report = RuboCop::Cop::Team.new([klass.new(config, autocorrect: true)], config, raise_error: true).investigate(source)
    {"id" => "#{style}: #{sample}", "original" => text, "config" => options, "ruby_diagnostics" => report.offenses.reject(&:disabled?).sort.map { |o| {line: o.line, column: o.column + 1, length: o.location.length, message: o.message} }, "ruby_corrected" => report.correctors.compact.find { |c| !c.empty? }&.rewrite || text}
  end
end
File.write(ARGV.fetch(0), JSON.pretty_generate(rows) + "\n")
