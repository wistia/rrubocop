# frozen_string_literal: true

require "json"
require "rspec/mocks/standalone"

module ReturnSemanticCurrent
  VALUE = :initial
end

class ReturnSemanticProbe
  include RSpec::Mocks::ExampleMethods

  def immutable(block_style:)
    RSpec::Mocks.with_temporary_scope do
      object = Object.new
      if block_style
        allow(object).to receive(:run) { 42 }
      else
        allow(object).to receive(:run).and_return(42)
      end
      [object.run, object.run]
    end
  end

  def mutable(block_style:)
    RSpec::Mocks.with_temporary_scope do
      object = Object.new
      if block_style
        allow(object).to receive(:run) { [] }
      else
        allow(object).to receive(:run).and_return([])
      end
      Array.new(2) do
        value = object.run
        before = value.dup
        value << :changed
        before
      end
    end
  end

  def constant(block_style:)
    ReturnSemanticCurrent.send(:remove_const, :VALUE)
    ReturnSemanticCurrent.const_set(:VALUE, :initial)
    RSpec::Mocks.with_temporary_scope do
      object = Object.new
      if block_style
        allow(object).to receive(:run) { ReturnSemanticCurrent::VALUE }
      else
        allow(object).to receive(:run).and_return(ReturnSemanticCurrent::VALUE)
      end
      ReturnSemanticCurrent.send(:remove_const, :VALUE)
      ReturnSemanticCurrent.const_set(:VALUE, :updated)
      object.run
    end
  end

  def optional_argument(block_style:)
    RSpec::Mocks.with_temporary_scope do
      object = Object.new
      events = []
      if block_style
        allow(object).to receive(:run) { |value = (events << :default)| 42 }
      else
        allow(object).to receive(:run).and_return(42)
      end
      {returned: object.run, events:}
    end
  end
end

probe = ReturnSemanticProbe.new
immutable = {block: probe.immutable(block_style: true), and_return: probe.immutable(block_style: false)}
raise "Immutable response changed" unless immutable[:block] == immutable[:and_return]
mutable = {block: probe.mutable(block_style: true), and_return: probe.mutable(block_style: false)}
raise "Fresh response hazard missing" unless mutable[:block] == [[], []] && mutable[:and_return] == [[], [:changed]]
constant = {block: probe.constant(block_style: true), and_return: probe.constant(block_style: false)}
raise "Constant lookup timing hazard missing" unless constant == {block: :updated, and_return: :initial}
optional_argument = {block: probe.optional_argument(block_style: true), and_return: probe.optional_argument(block_style: false)}
raise "Optional parameter effect missing" unless optional_argument[:block] == {returned: 42, events: [:default]} && optional_argument[:and_return] == {returned: 42, events: []}
puts JSON.pretty_generate({immutable:, mutable:, constant:, optional_argument:})
