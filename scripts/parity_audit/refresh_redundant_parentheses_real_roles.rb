# frozen_string_literal: true

require "bundler/setup"
require "rubocop"
require "json"
klass = RuboCop::Cop::Style::RedundantParentheses
samples = [
  "obj.x = (foo(1))",
  "obj.x = (1)",
  "x = (foo(1))",
  "obj[1] = (foo || bar)",
  "obj[1] = (1)",
  "landing =~ (/foo/)",
  "x == (1)",
  "x + (1)",
  "x ** (1)",
  "(/foo/) =~ x",
  "expect(X).to receive(:x).and_wrap_original do |method| method.call end.at_least(:once)",
  "foo((1))",
  "foo (1)",
  "d.x = (a(1))..\n (b(2))",
  "a..\n (b(2))",
  "a...\n (b(2))",
  "a..; (b(2))",
  "a..\n\n (b(2))",
  "a.. # retain\n (b(2))"
]
rows = samples.flat_map do |sample|
  [3.4].filter_map do |version|
    text = sample + "\n"
    options = RuboCop::ConfigLoader.default_configuration.for_cop(klass.cop_name).merge("Enabled" => true, "TargetRubyVersion" => version)
    config = RuboCop::Config.new("AllCops" => {"TargetRubyVersion" => version}, klass.cop_name => options)
    source = RuboCop::ProcessedSource.new(text, version)
    next unless source.valid_syntax?
    source.config = config
    source.registry = RuboCop::Cop::Registry.new([klass])
    report = RuboCop::Cop::Team.new([klass.new(config, autocorrect: true)], config, raise_error: true).investigate(source)
    {"id" => "#{version}: #{sample}", "original" => text, "config" => options, "ruby_diagnostics" => report.offenses.reject(&:disabled?).sort.map { |o| {line: o.line, column: o.column + 1, length: o.location.length, message: o.message} }, "ruby_corrected" => report.correctors.compact.find { |c| !c.empty? }&.rewrite || text}
  end
end
File.write(ARGV.fetch(0), JSON.pretty_generate(rows) + "\n")
