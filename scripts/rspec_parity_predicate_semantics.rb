# frozen_string_literal: true

require "json"
require "rspec/expectations"

class PredicateSemanticProbe
  include RSpec::Matchers

  class NonBoolean
    def empty?
      :truthy
    end
  end

  def stable
    {original: be_truthy.matches?([].empty?), inflected: be_empty.matches?([]), explicit: be(true).matches?([].empty?)}
  end

  def strictness
    object = NonBoolean.new
    {original: be(true).matches?(object.empty?), inflected: be_empty.matches?(object)}
  end

  def eql_polarity
    {original: eql(true).matches?([].empty?), upstream: be_empty.does_not_match?([])}
  end

  def hash_inclusion
    {original: be_truthy.matches?({a: 1}.include?(a: 1)), inflected: include(a: 1).matches?({a: 1})}
  end

  def array_inclusion
    {original: be_truthy.matches?([1].include?(Integer)), inflected: include(Integer).matches?([1])}
  end

  def timing(inflected:)
    events = []
    object = Object.new
    object.define_singleton_method(:empty?) { events << :predicate; true }
    message = proc { events << :message; "failure" }
    if inflected
      expect(object).to be_empty, message.call
    else
      expect(object.empty?).to be_truthy, message.call
    end
    events
  end
end

probe = PredicateSemanticProbe.new
stable = probe.stable
raise "Stable builtin predicate changed" unless stable.values.all?
strictness = probe.strictness
raise "Strict truth-value hazard missing" if strictness[:original] == strictness[:inflected]
eql_polarity = probe.eql_polarity
raise "Original eql polarity hazard missing" if eql_polarity[:original] == eql_polarity[:upstream]
timing = {original: probe.timing(inflected: false), inflected: probe.timing(inflected: true)}
raise "Predicate timing hazard missing" if timing[:original] == timing[:inflected]
hash_inclusion = probe.hash_inclusion
array_inclusion = probe.array_inclusion
[hash_inclusion, array_inclusion].each do |result|
  raise "RSpec inclusion semantics hazard missing" if result[:original] == result[:inflected]
end
puts JSON.pretty_generate({stable:, strictness:, eql_polarity:, timing:, hash_inclusion:, array_inclusion:})
