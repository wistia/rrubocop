# frozen_string_literal: true

require_relative "../refresh_ported_reference"
require "timeout"
require_relative "reference/results"

class AuditReference < PortedReference
  include AuditResults

  def run(input, output)
    configure_autoload_fixture
    load_native(input)
    File.write(File.join(File.dirname(output), "reference-meta.json"), JSON.pretty_generate(reference_identity))
    File.open(output, "w") do |file|
      JSON.parse(File.read(input)).each_with_index { |test, index| write_case(file, test, index) }
    end
  end

  private

  def reference_identity
    {ruby: RUBY_VERSION, rubocop: RuboCop::Version::STRING, config: @config.loaded_path}
  end

  def audit(test, index)
    original = test.fetch("source")
    original_compile = compile(original)
    return {original_invalid: true, original_compile:} unless original_compile.fetch(:valid)

    parsed = source(test, index)
    results = {original_compile:, diagnostics: diagnostics(investigation(test, parsed, false)),
               native_compile: native_compile(test)}
    %w[safe all].each { |mode| results[mode] = correction(test, index, mode) }
    results
  end

  def load_native(input)
    rows = File.foreach(File.join(File.dirname(input), "native.jsonl")).map { |line| JSON.parse(line) }
    @native = rows.group_by { |row| row.fetch("id") }
  end

  def write_case(file, test, index)
    result = Timeout.timeout(5) { audit(test, index) }
    file.puts(JSON.generate(id: test.fetch("id"), result:))
    file.flush
  rescue StandardError, SyntaxError => e
    file.puts(JSON.generate(id: test.fetch("id"), result: {error: "#{e.class}: #{e.message}"}))
  end

  def native_compile(test)
    @native.fetch(test.fetch("id")).to_h do |row|
      [row.fetch("mode"), compile(row.fetch("result").fetch("source", test.fetch("source")))]
    end
  end

  def investigation(test, parsed, safe)
    config = configuration(test)
    parsed.config = config
    cop = cop_class(test.fetch("cop")).new(config, autocorrect: true, safe_autocorrect: safe)
    cops = supported_cops(cop)
    report = RuboCop::Cop::Commissioner.new(cops, RuboCop::Cop::Team.forces_for(cops),
      raise_error: true).investigate(parsed)
    raise report.errors.inspect unless report.errors.empty?

    report
  end

  def supported_cops(cop)
    return [] if cop.class.respond_to?(:support_target_ruby_version?) &&
      !cop.class.support_target_ruby_version?(cop.target_ruby_version)

    return [] unless RuboCop::Cop::Team.new([], cop.config).send(:support_target_rails_version?, cop)
    return [] unless cop.send(:target_satisfies_all_gem_version_requirements?)

    [cop]
  end

  def configuration(test)
    name = test.fetch("cop")
    options = @config.for_cop(name).merge("AutoCorrect" => true).merge(test.fetch("config", {})).merge(
      "Enabled" => true, "Include" => ["**/*"], "Exclude" => []
    )
    all_cops = @config.for_all_cops.merge(test.fetch("config", {}).slice("TargetRubyVersion", "TargetRailsVersion"))
    base = @config.merge(test.fetch("config", {})["GlobalConfig"] || {})
    config = RuboCop::Config.new(base.merge(name => options, "AllCops" => all_cops), @config.loaded_path)
    with_fixture_gem_versions(config, test.fetch("config", {})["GemVersions"])
  end

  def with_fixture_gem_versions(config, gems)
    return config unless gems

    versions = gems.transform_values { |version| Gem::Version.new(version.to_s) }
    config.instance_variable_set(:@gem_versions_in_target, versions)
    config
  end

  def correction(test, index, mode)
    state = {source: test.fetch("source"), seen: {}}
    10.times do
      result = correction_step(test, index, mode, state)
      return result if result
    end
    corrected(state.fetch(:source)).merge(error: "correction pass limit reached")
  end

  def correction_step(test, index, mode, state)
    report = investigation(test, source(test.merge("source" => state.fetch(:source)), index), mode == "safe")
    corrector = report.cop_reports.first&.corrector
    return corrected(state.fetch(:source)) if corrector.blank?

    advance(corrector.rewrite, state)
  end

  def advance(after, state)
    current = state.fetch(:source)
    if state.fetch(:seen)[after] || after == current
      return corrected(current).merge(error: "correction cycle or no progress")
    end

    state.fetch(:seen)[current] = true
    state[:source] = after
    compile(after).fetch(:valid) ? nil : corrected(after)
  end
end

AuditReference.new(ARGV.fetch(0), ARGV.fetch(1)).run(ARGV.fetch(1), ARGV.fetch(2)) if $PROGRAM_NAME == __FILE__
