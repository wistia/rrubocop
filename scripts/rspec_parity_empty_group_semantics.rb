# frozen_string_literal: true

require "json"
require "rspec/core"
require "rspec/expectations"
require "stringio"

results = {stable: {}, setup: {}, description: {}}
[false, true].each do |corrected|
  key = corrected ? :upstream_rewrite : :original
  events = []
  RSpec.describe "empty group control #{key}" do
    unless corrected
      context "unused declarations" do
        let(:unused) { events << :helper }
        before { events << :hook }
      end
    end
    it { results[:stable][key] = events.dup }
  end
  setup_events = []
  RSpec.describe "empty group setup #{key}" do
    unless corrected
      context "group setup" do
        setup_events << :registered
      end
    end
    it { results[:setup][key] = setup_events.dup }
  end
  description_events = []
  RSpec.describe "empty group description #{key}" do
    unless corrected
      description = proc { description_events << :description; "unused" }
      context(description.call) {}
    end
    it { results[:description][key] = description_events.dup }
  end
end
output = StringIO.new
status = RSpec::Core::Runner.run(["--format", "progress"], output, output)
raise output.string unless status.zero?
raise "Unused helper/hook control differs" unless results[:stable][:original] == results[:stable][:upstream_rewrite]
%i[setup description].each do |kind|
  raise "Registration side effect hazard missing" if results[kind][:original] == results[kind][:upstream_rewrite]
end
puts JSON.pretty_generate({results:, examples_passed: 6})
