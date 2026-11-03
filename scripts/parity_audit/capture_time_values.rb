# frozen_string_literal: true

require 'rubocop'
require 'rubocop-rails'
require 'json'
config = RuboCop::Config.new('AllCops' => { 'TargetRubyVersion' => 3.4 },
                             'Rails/TimeZone' => { 'Enabled' => true,
                                                   'EnforcedStyle' => 'strict' })
cop = RuboCop::Cop::Rails::TimeZone.new(config)
input = JSON.parse(File.read(ARGV.fetch(0)))
rows = input.fetch('cases').map do |row|
  argument = row.fetch('source')
  source = RuboCop::ProcessedSource.new("Time.new(#{argument})", 3.4, 'app/models/example.rb')
  node = source.ast&.arguments&.first
  { source: argument, type: node&.type, value: node.respond_to?(:value) ? node.value.to_s : nil,
    attach: cop.send(:attach_timezone_specifier?, node) }
end
puts JSON.pretty_generate(rubocop: Gem.loaded_specs.fetch('rubocop').version.to_s,
                          rubocop_rails: Gem.loaded_specs.fetch('rubocop-rails').version.to_s,
                          target_ruby_version: 3.4, cases: rows)
