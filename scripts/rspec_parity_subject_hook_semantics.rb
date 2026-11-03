# frozen_string_literal: true

require "json"
require "rspec/core"
require "rspec/expectations"
require "stringio"

subject_results = []
[false, true].each do |corrected|
  RSpec.describe "overwritten subject #{corrected}" do
    if corrected
      let(:first) { :first }
    else
      subject(:first) { :first }
      subject { raise "Overwritten lazy subject evaluated" }
    end
    subject(:last) { :last }
    it do
      subject_results << [first, last, subject]
      expect(subject_results.last).to eq([:first, :last, :last])
    end
  end
end

hook_results = []
[false, true].each do |corrected|
  RSpec.describe "proxy hook #{corrected}" do
    around { |example| example.run } unless corrected
    it { hook_results << :ran }
  end
end

class RegistrationProbe
  attr_reader :events

  def initialize
    @events = []
  end

  def around
    events << :registered
  end

  def run
    events << :worker_ran
  end
end

qualified = RegistrationProbe.new
qualified.around { |example| example.run }
raise "Qualified registration side effect disappeared" unless qualified.events == [:registered]

worker = RegistrationProbe.new
proxy = RegistrationProbe.new
callback = proc { worker.run }
callback.call(proxy)
raise "Different callback receiver effect disappeared" unless worker.events == [:worker_ran] && proxy.events.empty?

scope_events = []
scope = proc {
  scope_events << :scope_evaluated
  :each
}
scope.call
raise "Scope side effect disappeared" unless scope_events == [:scope_evaluated]

output = StringIO.new
status = RSpec::Core::Runner.run(["--format", "progress"], output, output)
raise output.string unless status.zero?
raise "Subject result changed" unless subject_results[0] == subject_results[1]
raise "Proxy hook changed example execution" unless hook_results == [:ran, :ran]

puts JSON.pretty_generate({subject_results:, hook_results:, qualified_registration: qualified.events,
                          guarded_worker: worker.events, guarded_scope: scope_events, examples_passed: 4})
