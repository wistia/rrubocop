# frozen_string_literal: true

require "json"

class MetadataOrderProbe
  def initialize
    @events = []
  end

  def record(value)
    @events << value
    value
  end

  def it(_description, **metadata)
    {events: @events.dup, metadata:}
  end

  def outcome(mode)
    if mode == :ruby_correction
      it "x", a: record(:a), z: record(:z)
    else
      it "x", z: record(:z), a: record(:a)
    end
  end
end

observations = {}
%i[original rust ruby_correction].each do |mode|
  observations.store(mode, MetadataOrderProbe.new.outcome(mode))
end
raise "Rust changed evaluation order" unless observations[:original] == observations[:rust]
raise "Expected upstream reordering" if observations[:original] == observations[:ruby_correction]

puts JSON.pretty_generate(observations)
