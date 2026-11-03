# frozen_string_literal: true

require "bundler/setup"
require "rubocop"
require "json"
cases = JSON.parse(File.read(ARGV.fetch(0)))
selected = (ARGV[2] == "--all") ? cases : cases.select { |entry| entry["id"].match?(/multiline-regexp|multiline-symbol|(?:continued-)?percent-word/) }
results = selected.map do |entry|
  observations = {}
  stages = {"before" => entry["original"], "original_corrected" => entry["ruby_corrected"]}
  stages["native_expected"] = entry["rust_corrected"] if entry["rust_corrected"]
  stages.each do |stage, source|
    processed = RuboCop::ProcessedSource.new(source, entry.fetch("config").fetch("TargetRubyVersion"))
    observations[stage] = {parser_errors: processed.diagnostics.map { |d| d.reason.to_s }, literal_values: processed.ast&.each_node(:str, :sym)&.map { |node| [node.type.to_s, node.children.map(&:inspect)] }}
  end
  abort "Native expected output changes literal values" if observations["native_expected"] && observations["before"] != observations["native_expected"]
  {id: entry["id"], source: entry["original"], corrected: entry["ruby_corrected"], native_expected: entry["rust_corrected"], observations: observations}
end
File.write(ARGV.fetch(1), JSON.pretty_generate(results) + "\n")
