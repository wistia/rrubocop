# frozen_string_literal: true

require "json"
require "rspec/core"
require "rspec/expectations"
require "stringio"

observations = []
RSpec.shared_examples "semantic inclusion" do
  before { observations << described_class }
  it("keeps helper behavior") { expect(value).to eq(42) }
end

RSpec.describe Array do
  it_should_behave_like "semantic inclusion" do
    let(:value) { 42 }
  end
end
RSpec.describe Hash do
  it_behaves_like "semantic inclusion" do
    let(:value) { 42 }
  end
end
RSpec.describe "predicate matcher semantics" do
  it("original") { expect([42]).to be_include(42) }
  it("corrected") { expect([42]).to include(42) }
end

output = StringIO.new
status = RSpec::Core::Runner.run(["--format", "progress"], output, output)
raise output.string unless status.zero?
raise "Shared-example setup changed" unless observations.sort_by(&:name) == [Array, Hash]

puts JSON.pretty_generate({shared_group_setup: observations.map(&:name).sort, examples_passed: 4})
