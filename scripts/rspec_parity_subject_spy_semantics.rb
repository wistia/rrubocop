# frozen_string_literal: true

require "json"
require "rspec/core"
require "rspec/expectations"
require "rspec/mocks/standalone"
require "stringio"

class SpySemanticTarget
  def ping
    :pong
  end
end

class SpySemanticProbe
  include RSpec::Mocks::ExampleMethods

  def outcome(spy:)
    RSpec::Mocks.with_temporary_scope do
      object = spy ? instance_spy(SpySemanticTarget) : instance_double(SpySemanticTarget).as_null_object
      return_value = object.ping
      expect(object).to have_received(:ping).once
      {returns_self: return_value.equal?(object), received_once: true}
    end
  end
end

probe = SpySemanticProbe.new
spy = {original: probe.outcome(spy: false), corrected: probe.outcome(spy: true)}
raise "Spy behavior changed" unless spy[:original] == spy[:corrected]

subject_events = []
RSpec.describe "subject semantics" do
  subject do
    subject_events << :constructed
    42
  end

  it("implicit") { is_expected.to eq(42) }
  it("explicit") { expect(subject).to eq(42) }
end

output = StringIO.new
status = RSpec::Core::Runner.run(["--format", "progress"], output, output)
raise output.string unless status.zero?
raise "Subject evaluated incorrectly" unless subject_events == [:constructed, :constructed]

puts JSON.pretty_generate({spy:, subject_events:, examples_passed: 2})
