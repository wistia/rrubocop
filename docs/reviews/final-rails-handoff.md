# Final Rails handoff

Branch `feat/wistia-parity-rails-final` starts at framework `9e7b524` and owns
only Delegate, FilePath and HttpPositionalArguments. Earlier RSpec, Performance
and Schema branches are unchanged. See `final-rails-parity.md` and its JSON for
source review, complete strict failures and bounded claims.

Integrate the three cop files, isolated `final_rails_*` scripts/fixtures/reports,
case/scope import hooks and the existing Ruby literal formatter's thin
`string_inspect` wrapper. Schema already exposed an identical wrapper, so retain
one definition when merging. Preserve framework fixture unions and newer hooks.
The inherited HttpStatus change is rustfmt-only. No shared decoder/model/engine
or generic scheduling work is part of this branch.

The strict driver deliberately exits nonzero for retained original crashes,
unsafe original transformations and correction safeguards. It has no allowlist.
Compare diagnostic fields separately from the final correctability flag when
reviewing the report; guards change metadata intentionally. Runtime execution is
restricted to authored programs, never the imported upstream examples.

Reproduce the owned gate using `final_rails_cases.cases` as the driver case
provider with scripts/framework_parity/check.py. Production scopes use the usual
scope_check.py with the three --cop selections. Physical absent/lower/conflicting
railties locks use final_rails_lockfile_check.py. The report generator reads the
strict driver's original artifacts without suppressing differences.

No release, S3 publication or rollout modification is included. Combined-cop
scheduling remains explicitly deferred.

Final results: 547 originals/1,094 generated states; zero detection/message/range/
severity differences; 26 trusted programs/52 native modes preserve behavior.
Native 934 passed/one ignored; production scope 11 exact; physical locks eight
exact. All 316 retained strict failures are detailed in the JSON: guard metadata/
bytes, original exceptions/percent-literal loops and ten unsafe original runtime
observations. No invalid native correction or idempotence failure remains.
