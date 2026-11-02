# frozen_string_literal: true

require "json"
require "rspec/core"

class MetadataStyleSemanticProbe
  def self.flags(group)
    group.metadata.slice(:feature, :other)
  end
end

symbol_original = MetadataStyleSemanticProbe.flags(RSpec.describe("symbol original", feature: true, other: 1) {})
symbol_corrected = MetadataStyleSemanticProbe.flags(RSpec.describe("symbol corrected", :feature, other: 1) {})
raise "Symbol metadata conversion changed flags" unless symbol_original == symbol_corrected
hash_original = MetadataStyleSemanticProbe.flags(RSpec.describe("hash original", :feature, other: 1) {})
hash_corrected = MetadataStyleSemanticProbe.flags(RSpec.describe("hash corrected", other: 1, feature: true) {})
raise "Hash metadata conversion changed flags" unless hash_original == hash_corrected

conflicting = {feature: false}
conflicting[:feature] = true
explicit_original = MetadataStyleSemanticProbe.flags(RSpec.describe("symbol overrides hash", :feature, feature: false) {})
explicit_corrected = MetadataStyleSemanticProbe.flags(RSpec.describe("pair appended", **conflicting) {})
raise "RSpec symbol precedence changed" unless explicit_original[:feature] == true && explicit_corrected[:feature] == true

overridden = {feature: true}
overridden[:feature] = false
later_original = MetadataStyleSemanticProbe.flags(RSpec.describe("later false original", **overridden) {})
later_rewrite = MetadataStyleSemanticProbe.flags(RSpec.describe("later false rewrite", :feature, feature: false) {})
raise "Later override hazard missing" unless later_original[:feature] == false && later_rewrite[:feature] == true

metadata = {feature: false, other: 1}
spread_original = MetadataStyleSemanticProbe.flags(RSpec.describe("spread original", feature: true, **metadata) {})
spread_rewrite = MetadataStyleSemanticProbe.flags(RSpec.describe("spread rewrite", :feature, **metadata) {})
raise "Spread override hazard missing" unless spread_original[:feature] == false && spread_rewrite[:feature] == true

puts JSON.pretty_generate({rspec_core_version: Gem.loaded_specs.fetch("rspec-core").version.to_s,
                          symbol_original:, symbol_corrected:, hash_original:, hash_corrected:,
                          explicit_original:, explicit_corrected:, later_original:, later_rewrite:,
                          spread_original:, spread_rewrite:})
