# frozen_string_literal: true

require "json"
require "rspec/mocks/standalone"

class CountExpectationProbe
  include RSpec::Mocks::ExampleMethods

  def outcome(mode, calls)
    RSpec::Mocks.with_temporary_scope do
      object = double("count probe")
      case mode
      when :original
        expect(object).to receive(:message).exactly(0x1).times
      when :rust
        expect(object).to receive(:message).once
      when :ruby_correction
        expect(object).to receive(:message).twice
      end
      calls.times { object.message }
    end
    true
  rescue RSpec::Mocks::MockExpectationError
    false
  end
end

probe = CountExpectationProbe.new
observations = (0..2).map do |calls|
  original = probe.outcome(:original, calls)
  rust = probe.outcome(:rust, calls)
  ruby_correction = probe.outcome(:ruby_correction, calls)
  raise "Rust changed the expectation" unless original == rust

  {calls:, original:, rust:, ruby_correction:}
end
raise "Expected an upstream behavior difference" unless observations.any? { |row| row[:original] != row[:ruby_correction] }

puts JSON.pretty_generate(observations)
