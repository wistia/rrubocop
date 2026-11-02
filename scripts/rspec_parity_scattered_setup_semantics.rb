# frozen_string_literal: true

require "json"
require "rspec/core"
require "rspec/expectations"
require "stringio"

results = %i[before after around locals eager heredoc duplicates].to_h { |kind| [kind, {}] }
results.each do |kind, versions|
  [false, true].each do |merged|
    events = []
    versions[merged ? :upstream : :original] = events
    RSpec.describe "#{kind} #{merged}" do
      case kind
      when :before
        if merged
          before { events << :first; events << :second }
        else
          before { events << :first }
          before { events << :second }
        end
      when :after
        if merged
          after { events << :first; events << :second }
        else
          after { events << :first }
          after { events << :second }
        end
      when :around
        if merged
          around do |example|
            events << :first_before
            example.run
            events << :first_after
            events << :second_before
            example.run
            events << :second_after
          end
        else
          around { |example| events << :first_before; example.run; events << :first_after }
          around { |example| events << :second_before; example.run; events << :second_after }
        end
      when :locals
        if merged
          before do
            value = 1
            events << defined?(value)
          end
        else
          before { value = 1 }
          before { events << defined?(value) }
        end
      when :eager
        if merged
          before { events << :first; events << :second }
          let!(:eager) { events << :eager }
        else
          before { events << :first }
          let!(:eager) { events << :eager }
          before { events << :second }
        end
      when :duplicates
        if merged
          before { events << :first; events << :second; events << :first }
        else
          before { events << :first }
          before { events << :first }
          before { events << :second }
        end
      when :heredoc
        if merged
          before do
            events << :first
            events << <<~TEXT
              preserved
            TEXT
          end
        else
          before { events << :first }
          before do
            events << <<~TEXT
              preserved
            TEXT
          end
        end
      end
      it { events << :example }
    end
  end
end
output = StringIO.new
status = RSpec::Core::Runner.run(["--format", "progress"], output, output)
raise output.string unless status.zero?
%i[before heredoc].each do |kind|
  raise "Safe callback combination changed behavior" unless results[kind][:original] == results[kind][:upstream]
end
%i[after around locals eager duplicates].each do |kind|
  raise "Callback grouping hazard missing" if results[kind][:original] == results[kind][:upstream]
end
puts JSON.pretty_generate({results:, examples_passed: 14})
