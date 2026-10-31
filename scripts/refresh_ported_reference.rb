# frozen_string_literal: true

require 'json'
require 'pathname'
require 'rubocop'

# Replays the Rust fixtures through the project's installed Ruby cops.
class PortedReference
  METADATA = %w[Description StyleGuide Reference Enabled VersionAdded VersionChanged Include Exclude].freeze

  def initialize(config_path)
    @config = RuboCop::ConfigLoader.configuration_from_file(config_path)
    @repository = Pathname.new(__dir__).parent
    @fixture_file = @repository.join('tests/fixtures/ported-reference.json')
  end

  def refresh
    configure_autoload_fixture
    results = JSON.parse(File.read(@fixture_file)).map.with_index do |test, index|
      refresh_case(test, index)
    end
    File.write(@fixture_file, "#{JSON.pretty_generate(results)}\n")
    puts "Refreshed #{results.length} examples with RuboCop #{RuboCop::Version::STRING}."
  end

  private

  def project
    @repository.join('tests/fixtures/ported_project')
  end

  def configure_autoload_fixture
    return unless defined?(Cops::NoRequireOnAutoLoadedPaths)

    Cops::NoRequireOnAutoLoadedPaths.send(:remove_const, :PATHS)
    Cops::NoRequireOnAutoLoadedPaths.const_set(:PATHS, [project.join('app/models').to_s])
  end

  def refresh_case(test, index)
    options = @config.for_cop(test.fetch('cop')).merge(test.fetch('options', {}))
    report = investigate(test, index, options)
    test.merge('config' => snapshot_options(options),
               'expected' => offenses(report, test.fetch('cop')),
               'corrected' => corrected_source(report, test.fetch('source')))
  end

  def snapshot_options(options)
    options.reject { |key, _| METADATA.include?(key) }.merge('TargetRubyVersion' => 3.4,
                                                             'ActiveSupportExtensionsEnabled' => @config.active_support_extensions_enabled?)
  end

  def investigate(test, index, options)
    report = RuboCop::Cop::Commissioner.new([configured_cop(test.fetch('cop'), options)], [],
                                            raise_error: true).investigate(source(test, index))
    raise report.errors.inspect unless report.errors.empty?

    report
  end

  def configured_cop(name, options)
    cop_class(name).new(RuboCop::Config.new(@config.merge(name => options), @config.loaded_path), autocorrect: true)
  end

  def cop_class(name)
    RuboCop::Cop::Registry.global.find_by_cop_name(name) || raise("Unavailable Ruby cop: #{name}")
  end

  def source(test, index)
    source = RuboCop::ProcessedSource.new(test.fetch('source'), 3.4,
                                          test.fetch('path').sub('$PROJECT', project.to_s))
    raise "Invalid Ruby syntax in case #{index}" unless source.valid_syntax?

    source
  end

  def offenses(report, name)
    report.offenses.map do |offense|
      { line: offense.line, column: offense.column,
        message: offense.message.delete_prefix("#{name}: "), correctable: offense.correctable? }
    end
  end

  def corrected_source(report, original)
    corrector = report.cop_reports.first.corrector
    corrector && !corrector.empty? ? corrector.rewrite : original
  end
end

PortedReference.new(ARGV.fetch(0, '.rubocop.yml')).refresh if $PROGRAM_NAME == __FILE__
