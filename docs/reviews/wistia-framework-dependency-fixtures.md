# Original framework dependency fixtures

Pinned reference: RuboCop 1.65.1, rubocop-rails 2.26.0. `Rails/IndexWith.minimum_target_rails_version 6.0` uses `requires_gem 'railties', '>= 6.0'` on this version. Original `Cop::Base#target_satisfies_all_gem_version_requirements?` checks `Config#gem_versions_in_target`; overriding `AllCops.TargetRailsVersion` alone does not change that dependency dictionary.

The Commissioner harness now calls that original dependency predicate before investigation, matching Base's normal exclusion policy. Optional `GemVersions` test dictionaries populate the same config cache that upstream RSpec's `stub_gem_versions` replaces. Native audit fixtures map the explicit railties dependency to existing resolved-config lockfile presence/version fields; production configuration is unchanged. String version values are the fixture schema.

Index fixtures cover absent railties, Rails 5.2, 6.0 and 8.1, plus conflicting target/dependency versions. The full regression completed 3,308 original cases, 6,616 second passes and 75 trusted programs with exactly the previously expected safeguard events; the expanded Index matrix independently covers 252 cases, 504 second passes and 12 trusted programs. No exception list hides differences.

A separate malformed-source harness probe now produces three explicit `original_invalid` events (original plus both second-pass states) and complete artifacts, instead of aborting on a missing `diagnostics` key. The probe is parsed/compiled only. Ruby harness lint passes.
