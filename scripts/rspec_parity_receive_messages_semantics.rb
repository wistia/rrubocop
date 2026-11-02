# frozen_string_literal: true

require "json"
require "rspec/mocks/standalone"

class StubSemanticProbe
  include RSpec::Mocks::ExampleMethods

  def stable(merged:)
    RSpec::Mocks.with_temporary_scope do
      object = Object.new
      if merged
        allow(object).to receive_messages(foo: 1, bar: 2)
      else
        allow(object).to receive(:foo).and_return(1)
        allow(object).to receive(:bar).and_return(2)
      end
      [object.foo, object.bar]
    end
  end

  def dependency(merged:)
    RSpec::Mocks.with_temporary_scope do
      object = Object.new
      object.define_singleton_method(:foo) { :before }
      if merged
        allow(object).to receive_messages(foo: 1, bar: object.foo)
      else
        allow(object).to receive(:foo).and_return(1)
        allow(object).to receive(:bar).and_return(object.foo)
      end
      [object.foo, object.bar]
    end
  end

  def intervening_assignment(merged:)
    RSpec::Mocks.with_temporary_scope do
      object = Object.new
      value = 1
      if merged
        value = 2
        allow(object).to receive_messages(foo: value, bar: 3)
      else
        allow(object).to receive(:foo).and_return(value)
        value = 2
        allow(object).to receive(:bar).and_return(3)
      end
      [object.foo, object.bar]
    end
  end

  def receiver_calls(merged:)
    RSpec::Mocks.with_temporary_scope do
      object = Object.new
      calls = 0
      provider = proc { calls += 1; object }
      if merged
        allow(provider.call).to receive_messages(foo: 1, bar: 2)
      else
        allow(provider.call).to receive(:foo).and_return(1)
        allow(provider.call).to receive(:bar).and_return(2)
      end
      {calls:, values: [object.foo, object.bar]}
    end
  end
end

probe = StubSemanticProbe.new
results = %i[stable dependency intervening_assignment receiver_calls].to_h do |kind|
  [kind, {original: probe.public_send(kind, merged: false), upstream: probe.public_send(kind, merged: true)}]
end
raise "Literal stub merge changed values" unless results[:stable][:original] == results[:stable][:upstream]
%i[dependency intervening_assignment receiver_calls].each do |kind|
  raise "Stub grouping hazard missing" if results[kind][:original] == results[kind][:upstream]
end
puts JSON.pretty_generate(results)
