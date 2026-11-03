# Parity integration checkpoint

All seven frozen completed source snapshots are merged on
`feat/wistia-parity-integrated`. The exact snapshot SHAs, merge decisions and
validation provenance are recorded in `wistia-parity-integration.json`.
Core's separately committed numeric-span helper d5eb3e6 is included; its
uncommitted FloatOutOfRange consumer is excluded. All original branches remain
unchanged.

The merged non-test library check passed after the seven merges. The first
integrated full native run is in progress and has exposed an RSpec documented
safety-fixture failure requiring investigation. This checkpoint does not claim
integrated test success. Fresh family differentials, actual scope/cache/config
controls and the initial 553-enabled-rule/160-stub inventory comparison follow.
Earlier family reports describe their own branch tests, not this integrated tree.

Shared resolution preserves the newer AST/typed-literal APIs and golden unions,
Core decoder wrappers, synthetic-read filtering plus cached binding identities,
RSpec heredoc/inspect APIs, original Team/gem/global/schema harness contexts,
all source/runtime safeguards and the user-deferred scheduling boundary.

The first full run completed 1,080 tests and exposed four RSpec failures from one
shared-model interaction: the old lexical numbered-block scan depended on bare
method classification. It now uses existing cached numbered binding metadata.
All 49 focused RSpec tests pass, including the strict original and safeguard
corpora, with no fixture expectation changes. Full-suite rerun and fresh original
reference regeneration remain pending.
