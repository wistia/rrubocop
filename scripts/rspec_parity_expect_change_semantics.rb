# frozen_string_literal: true

require "json"
require "rspec/expectations"
require "rspec/mocks/standalone"

class ChangeStyleCounter
  attr_accessor :value

  def initialize(value)
    @value = value
  end
end

class ChangeStyleBefore
  def self.value
    0
  end
end

class ChangeStyleAfter
  def self.value
    1
  end
end

class ChangeStyleSemanticProbe
  include RSpec::Matchers
  include RSpec::Mocks::ExampleMethods

  def stable(block_style:)
    counter = ChangeStyleCounter.new(0)
    matcher = block_style ? change { counter.value } : change(counter, :value)
    {matches: matcher.by(1).matches?(proc { counter.value += 1 }), final: counter.value}
  end

  def receiver_calls(block_style:)
    calls = []
    counter = ChangeStyleCounter.new(0)
    provider = proc {
      calls << :receiver
      counter
    }
    matcher = block_style ? change { provider.call.value } : change(provider.call, :value)
    matched = matcher.by(1).matches?(proc { counter.value += 1 })
    {matched:, calls:}
  end

  def reassignment(block_style:)
    counter = ChangeStyleCounter.new(0)
    matcher = block_style ? change { counter.value } : change(counter, :value)
    matched = matcher.by(1).matches?(proc { counter = ChangeStyleCounter.new(1) })
    {matched:}
  end

  def mutable_literal(block_style:)
    literal = 0
    matcher = block_style ? change { "value#{literal}".object_id } : change("value#{literal}", :object_id)
    {matched: matcher.matches?(proc {})}
  end

  def constant_swap(block_style:)
    RSpec::Mocks.with_temporary_scope do
      stub_const("ChangeStyleCurrent", ChangeStyleBefore)
      matcher = block_style ? change { ChangeStyleCurrent.value } : change(ChangeStyleCurrent, :value)
      {matched: matcher.by(1).matches?(proc { stub_const("ChangeStyleCurrent", ChangeStyleAfter) })}
    end
  end

  def literal_size(block_style:)
    matcher = block_style ? change { "value".length } : change("value", :length)
    {matched: matcher.by(0).matches?(proc {})}
  end
end

probe = ChangeStyleSemanticProbe.new
stable = {block: probe.stable(block_style: true), arguments: probe.stable(block_style: false)}
raise "Stable change receiver changed behavior" unless stable[:block] == stable[:arguments]
calls = {block: probe.receiver_calls(block_style: true), arguments: probe.receiver_calls(block_style: false)}
raise "Receiver evaluation hazard missing" unless calls[:block][:calls].size == 2 && calls[:arguments][:calls].size == 1
reassignment = {block: probe.reassignment(block_style: true), arguments: probe.reassignment(block_style: false)}
raise "Capture hazard missing" unless reassignment[:block][:matched] && !reassignment[:arguments][:matched]
mutable_literal = {block: probe.mutable_literal(block_style: true), arguments: probe.mutable_literal(block_style: false)}
raise "Fresh receiver hazard missing" unless mutable_literal[:block][:matched] && !mutable_literal[:arguments][:matched]
literal_size = {block: probe.literal_size(block_style: true), arguments: probe.literal_size(block_style: false)}
raise "Stable literal getter changed behavior" unless literal_size[:block] == literal_size[:arguments]
constant_swap = {block: probe.constant_swap(block_style: true), arguments: probe.constant_swap(block_style: false)}
raise "Constant replacement hazard missing" unless constant_swap[:block][:matched] && !constant_swap[:arguments][:matched]
puts JSON.pretty_generate({stable:, calls:, reassignment:, mutable_literal:, literal_size:, constant_swap:})
