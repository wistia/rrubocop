# frozen_string_literal: true

require "json"
require "rspec/mocks/standalone"

class SingleChainSemanticProbe
  include RSpec::Mocks::ExampleMethods

  def outcome(mode, response)
    RSpec::Mocks.with_temporary_scope do
      object = double("chain target")
      case mode
      when :hash_chain
        allow(object).to receive_message_chain(message: response)
      when :corrected_hash
        allow(object).to receive(:message).and_return(response)
      when :array_chain
        allow(object).to receive_message_chain([:message]).and_return(response)
      when :corrected_array
        allow(object).to receive(:message).and_return(response)
      end
      [object.message, object.message]
    end
  end
end

probe = SingleChainSemanticProbe.new
observations = [nil, 42, "value"].map do |response|
  outcomes = {}
  %i[hash_chain corrected_hash array_chain corrected_array].each do |mode|
    outcomes.store(mode, probe.outcome(mode, response))
  end
  raise "Chain response changed" unless outcomes.values.uniq.one?

  {response:, outcomes:}
end

puts JSON.pretty_generate(observations)
