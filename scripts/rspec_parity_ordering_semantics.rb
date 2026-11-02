# frozen_string_literal: true

require "json"
require "rspec/core"
require "rspec/expectations"
require "stringio"

results = {stable: {}, eager: {}, collision: {}, includes: {}}
[false, true].each do |corrected|
  key = corrected ? :upstream_rewrite : :original
  RSpec.describe "stable subject #{key}" do
    if corrected
      subject { [:subject, params] }
      let(:params) { :params }
    else
      let(:params) { :params }
      subject { [:subject, params] }
    end
    it { results[:stable][key] = subject }
  end

  events = []
  RSpec.describe "eager subject #{key}" do
    if corrected
      subject! { events << :subject }
      before { events << :before }
    else
      before { events << :before }
      subject! { events << :subject }
    end
    it { results[:eager][key] = events.dup }
  end

  RSpec.describe "subject collision #{key}" do
    if corrected
      subject(:value) { :subject }
      let(:value) { :helper }
    else
      let(:value) { :helper }
      subject(:value) { :subject }
    end
    it { results[:collision][key] = [value, subject] }
  end

  RSpec.describe "included subject #{key}" do
    shared_examples "included declaration" do
      subject(:value) { :included }
    end
    if corrected
      subject(:value) { :local }
      include_examples "included declaration"
    else
      include_examples "included declaration"
      subject(:value) { :local }
    end
    it { results[:includes][key] = value }
  end
end

%i[stable_hooks stable_helpers hook_order let_order scattered_order].each do |kind|
  results[kind] = {}
  [false, true].each do |corrected|
    key = corrected ? :upstream_rewrite : :original
    events = []
    RSpec.describe "#{kind} #{key}" do
      case kind
      when :stable_hooks
        before { events << :before } if corrected
        it { results[kind][key] = events.dup }
        before { events << :before } unless corrected
      when :stable_helpers
        let(:first) { :first }
        let(:second) { :second } if corrected
        it { results[kind][key] = [first, second] }
        let(:second) { :second } unless corrected
      when :hook_order
        before { events << :before } if corrected
        it { results[kind][key] = events.dup }
        let!(:value) { events << :eager }
        before { events << :before } unless corrected
      when :let_order
        let!(:value) { events << :eager } if corrected
        it { results[kind][key] = events.dup }
        before { events << :before }
        let!(:value) { events << :eager } unless corrected
      when :scattered_order
        let(:first) { :first }
        let!(:value) { events << :eager } if corrected
        before { events << :before }
        let!(:value) { events << :eager } unless corrected
        it { results[kind][key] = events.dup }
      end
    end
  end
end

results[:multiple_hooks] = {}
%i[original native upstream_rewrite].each do |mode|
  events = []
  RSpec.describe "multiple hooks #{mode}" do
    if mode == :upstream_rewrite
      before { events << :second }
      before { events << :first }
    elsif mode == :native
      before { events << :first }
      before { events << :second }
    end
    it { results[:multiple_hooks][mode] = events.dup }
    if mode == :original
      before { events << :first }
      before { events << :second }
    end
  end
end

output = StringIO.new
status = RSpec::Core::Runner.run(["--format", "progress", "--order", "defined"], output, output)
raise output.string unless status.zero?
raise "Stable hoisting changed behavior" unless results[:stable][:original] == results[:stable][:upstream_rewrite]
%i[eager collision includes].each do |kind|
  raise "Ordering hazard missing: #{kind}" if results[kind][:original] == results[kind][:upstream_rewrite]
end
%i[stable_hooks stable_helpers].each do |kind|
  raise "Safe declaration grouping changed behavior" unless results[kind][:original] == results[kind][:upstream_rewrite]
end
%i[hook_order let_order scattered_order].each do |kind|
  raise "Eager grouping hazard missing" if results[kind][:original] == results[kind][:upstream_rewrite]
end
raise "Native hook grouping changed order" unless results[:multiple_hooks][:original] == results[:multiple_hooks][:native]
raise "Upstream hook reversal hazard missing" if results[:multiple_hooks][:original] == results[:multiple_hooks][:upstream_rewrite]
puts JSON.pretty_generate({results:, examples_passed: 21})
