# frozen_string_literal: true

require "json"
require "rspec/expectations"

class ZeroChangeProbe
  include RSpec::Matchers

  def run(before, after, negated:)
    value = before
    matcher = change { value }
    matched = if negated
      matcher.does_not_match?(proc { value = after })
    else
      matcher.by(0).matches?(proc { value = after })
    end
    {matched:, before:, after:}
  end
end

probe = ZeroChangeProbe.new
stable = {by_zero: probe.run(1, 1, negated: false), negated: probe.run(1, 1, negated: true)}
raise "Stable integer control differs" unless stable[:by_zero] == stable[:negated]
representation = {by_zero: probe.run(1, 1.0, negated: false), negated: probe.run(1, 1.0, negated: true)}
raise "Representation/hash difference missing" unless representation[:by_zero][:matched] && !representation[:negated][:matched]
puts JSON.pretty_generate({stable:, representation:})
