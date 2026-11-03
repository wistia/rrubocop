# frozen_string_literal: true

require "bundler/setup"
require "rubocop"
require "json"
klass = RuboCop::Cop::Lint::UnderscorePrefixedVariableName
samples = ["items.map { _1 }", "items.map { [_1, _2, _3] }", "items.each do; puts _1; end", "-> { _1 }.call", "items.map { other.map { _1 }; _1 }", "_value = 1; puts _value", "items.map { |_value| _value }", "def f(_value); _value; end", "_value = 1", "items.map { |_1| _1 }", "_1 = 1; puts _1", "items.map { _1 = 1; puts _1 }"]
rows = samples.flat_map do |sample|
  [2.6, 2.7, 3.4].filter_map do |version|
    text = sample + "\n"
    options = RuboCop::ConfigLoader.default_configuration.for_cop(klass.cop_name).merge("Enabled" => true, "TargetRubyVersion" => version)
    config = RuboCop::Config.new("AllCops" => {"TargetRubyVersion" => version}, klass.cop_name => options)
    source = RuboCop::ProcessedSource.new(text, version)
    next unless source.valid_syntax?
    source.config = config
    source.registry = RuboCop::Cop::Registry.new([klass])
    report = RuboCop::Cop::Team.new([klass.new(config)], config, raise_error: true).investigate(source)
    {"id" => "#{version}: #{sample}", "original" => text, "config" => options, "ruby_diagnostics" => report.offenses.reject(&:disabled?).sort.map { |o| {line: o.line, column: o.column + 1, length: o.location.length, message: o.message} }, "ruby_corrected" => text}
  end
end
File.write(ARGV.fetch(0), JSON.pretty_generate(rows) + "\n")
