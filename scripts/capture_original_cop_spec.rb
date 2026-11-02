# frozen_string_literal: true

require "bundler/setup"
require "json"
require "rubocop"
require "rspec/core"
require "rspec/expectations"
require "rubocop/rspec/support"
RSpec.configure { |config| config.include RuboCop::RSpec::ExpectOffense }
spec_path, output_path = ARGV
abort "Usage: bundle exec ruby capture_original_cop_spec.rb SPEC_PATH OUTPUT_JSON" unless spec_path && output_path
module CaptureOperatorSpec
  def self.captured
    @captured ||= []
  end

  def _investigate(cop, processed_source)
    result = super
    config, siblings = captured_configuration(cop.class.cop_name)
    config["TargetRubyVersion"] = processed_source.ruby_version
    CaptureOperatorSpec.captured << {
      id: RSpec.current_example.full_description,
      original: processed_source.raw_source,
      path: processed_source.file_path,
      reference_version: RuboCop::Version::STRING,
      config:,
      sibling_cops: siblings,
      ruby_diagnostics: captured_diagnostics(result),
      ruby_safe: captured_correction(processed_source)
    }
    result
  end

  def captured_correction(processed_source)
    @last_corrector.empty? ? processed_source.raw_source : @last_corrector.rewrite
  end

  def captured_configuration(name)
    config = captured_base_options(name)
    siblings = {}
    case name
    when "Style/RedundantParentheses"
      capture_parentheses_options(config)
    when "Style/NonNilCheck"
      siblings = capture_non_nil_options(config)
    when "Style/ComparableClamp"
      config["IndentationWidth"] ||= configuration.for_cop("Layout/IndentationWidth")["Width"]
    when "Layout/ArrayAlignment"
      config["IndentationWidth"] ||= configuration.for_cop("Layout/IndentationWidth")["Width"]
    when "Layout/HashAlignment"
      argument_alignment = configuration.for_cop("Layout/ArgumentAlignment")
      config["ArgumentAlignmentEnabled"] = !!argument_alignment["Enabled"]
      config["ArgumentAlignmentStyle"] = argument_alignment["EnforcedStyle"]
      siblings = {"Layout/ArgumentAlignment" => argument_alignment.slice("Enabled", "EnforcedStyle")}
    end
    [config, siblings]
  end

  def captured_diagnostics(result)
    result.sort.map do |offense|
      {line: offense.line, column: offense.column + 1, length: offense.location.length, message: offense.message}
    end
  end

  def captured_base_options(name)
    config = configuration.for_cop(name).dup
    config["EnforcedHashRocketStyle"] = (configuration.for_cop("Layout/HashAlignment") || {})["EnforcedHashRocketStyle"]
    config["ForceEqualSignAlignment"] = (configuration.for_cop("Layout/ExtraSpacing") || {})["ForceEqualSignAlignment"]
    config["TargetRubyVersion"] = (configuration["AllCops"] || {})["TargetRubyVersion"]
    config
  end

  def capture_non_nil_options(config)
    nil_comparison = configuration.for_cop("Style/NilComparison")
    config["NilComparisonEnabled"] = nil_comparison["Enabled"]
    config["NilComparisonEnforcedStyle"] = nil_comparison["EnforcedStyle"]
    {"Style/NilComparison" => nil_comparison.slice("Enabled", "EnforcedStyle")}
  end

  def capture_parentheses_options(config)
    ternary = configuration.for_cop("Style/TernaryParentheses")
    config["TernaryParenthesesEnabled"] = ternary["Enabled"]
    config["TernaryParenthesesEnforcedStyle"] = ternary["EnforcedStyle"]
    condition = configuration.for_cop("Style/ParenthesesAroundCondition")
    config["ParenthesesAroundConditionEnabled"] = condition["Enabled"]
    config["AllowInMultilineConditions"] = condition["AllowInMultilineConditions"]
  end
end
CopHelper.prepend(CaptureOperatorSpec)
status = RSpec::Core::Runner.run([spec_path, "--options", "/dev/null", "--format", "progress"])
File.write(output_path, JSON.pretty_generate(CaptureOperatorSpec.captured.uniq { |entry|
  [entry[:original], entry[:config], entry[:path], entry[:reference_version]]
}) + "\n")
exit status
