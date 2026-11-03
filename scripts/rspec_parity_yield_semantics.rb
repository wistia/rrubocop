# frozen_string_literal: true

require "json"
require "rspec/mocks/standalone"

class YieldSemanticProbe
  include RSpec::Mocks::ExampleMethods

  def immutable(corrected:)
    RSpec::Mocks.with_temporary_scope do
      object = Object.new
      if corrected
        allow(object).to receive(:run).and_yield(1).and_yield(2)
      else
        allow(object).to receive(:run) { |&block|
                           block.call(1)
                           block.call(2)
                         }
      end
      yielded = []
      returned = object.run { |value|
        yielded << value
        value * 10
      }
      {yielded:, returned:}
    end
  end

  def mutable(corrected:)
    RSpec::Mocks.with_temporary_scope do
      object = Object.new
      if corrected
        allow(object).to receive(:run).and_yield([])
      else
        allow(object).to receive(:run) { |&block| block.call([]) }
      end
      observed = []
      2.times do
        object.run do |value|
          observed << value.dup
          value << :changed
        end
      end
      observed
    end
  end

  def timing(corrected:)
    RSpec::Mocks.with_temporary_scope do
      object = Object.new
      events = []
      value = proc {
        events << :argument
        1
      }
      if corrected
        allow(object).to receive(:run).and_yield(value.call)
      else
        allow(object).to receive(:run) { |&block| block.call(value.call) }
      end
      events << :configured
      object.run { |number| events << number }
      events
    end
  end
end

probe = YieldSemanticProbe.new
immutable = {original: probe.immutable(corrected: false), corrected: probe.immutable(corrected: true)}
raise "Immutable yields changed" unless immutable[:original] == immutable[:corrected]
mutable = {original: probe.mutable(corrected: false), upstream_rewrite: probe.mutable(corrected: true)}
raise "Fresh allocation hazard missing" unless mutable[:original] == [[], []] && mutable[:upstream_rewrite] == [[], [:changed]]
timing = {original: probe.timing(corrected: false), upstream_rewrite: probe.timing(corrected: true)}
raise "Evaluation timing hazard missing" unless timing[:original] == [:configured, :argument, 1] && timing[:upstream_rewrite] == [:argument, :configured, 1]
puts JSON.pretty_generate({immutable:, mutable:, timing:})
