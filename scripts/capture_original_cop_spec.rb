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
CAPTURED = []
module CaptureOperatorSpec
  def _investigate(cop, processed_source)
    result = super
    config = configuration.for_cop(cop.class.cop_name).dup
    config["EnforcedHashRocketStyle"] = configuration.for_cop("Layout/HashAlignment")["EnforcedHashRocketStyle"]
    config["ForceEqualSignAlignment"] = configuration.for_cop("Layout/ExtraSpacing")["ForceEqualSignAlignment"]
    config["TargetRubyVersion"] = configuration["AllCops"]["TargetRubyVersion"]
    CAPTURED << {
      id: RSpec.current_example.full_description,
      original: processed_source.raw_source,
      config: config,
      ruby_diagnostics: result.sort.map { |offense| {line: offense.line, column: offense.column + 1, length: offense.location.length, message: offense.message} },
      ruby_safe: @last_corrector.rewrite
    }
    result
  end
end
CopHelper.prepend(CaptureOperatorSpec)
status = RSpec::Core::Runner.run([spec_path, "--options", "/dev/null", "--format", "progress"])
File.write(output_path, JSON.pretty_generate(CAPTURED.uniq { |entry| [entry[:original], entry[:config]] }) + "\n")
exit status
