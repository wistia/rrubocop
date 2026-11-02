# frozen_string_literal: true

# Replays captured sources with fresh RuboCop instances to avoid retained ignored-node ranges.
require 'bundler/setup'
require 'rubocop'
require 'json'
input, output = ARGV
abort 'Usage: refresh_useless_assignment_states.rb INPUT_JSON OUTPUT_JSON' unless input && output
cases = JSON.parse(File.read(input))
cases.each do |entry|
  options = entry.fetch('config')
  config = RuboCop::Config.new('AllCops' => { 'TargetRubyVersion' => options.fetch('TargetRubyVersion') },
                               'Lint/UselessAssignment' => options)
  klass = RuboCop::Cop::Lint::UselessAssignment
  cop = klass.new(config, autocorrect: true)
  processed = RuboCop::ProcessedSource.new(entry.fetch('original'), options.fetch('TargetRubyVersion'))
  processed.config = config
  processed.registry = RuboCop::Cop::Registry.new([klass])
  report = RuboCop::Cop::Team.new([cop], config, raise_error: true).investigate(processed)
  entry['ruby_diagnostics'] = report.offenses.reject(&:disabled?).sort.map do |offense|
    { line: offense.line, column: offense.column + 1, length: offense.location.length, message: offense.message }
  end
  entry['ruby_corrected'] = (report.correctors.first || RuboCop::Cop::Corrector.new(processed)).rewrite
  entry.delete('ruby_safe')
  entry['reference_investigation'] = 'fresh cop per independent source/config'
end
File.write(output, "#{JSON.pretty_generate(cases)}\n")
