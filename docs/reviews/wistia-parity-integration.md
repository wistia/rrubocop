# Parity integration checkpoint

All seven frozen completed source snapshots are merged on
`feat/wistia-parity-integrated`. Exact snapshot SHAs and semantic merge decisions
are recorded in `wistia-parity-integration.json`; source branches remain preserved.
The validated source snapshot is c9d11a4. Core's tested numeric-span prerequisite
is included; its unfinished consumers are excluded.

The integrated suite passes 1,084 native tests with one ignored. One merge
interaction required replacing RSpec's legacy numbered-block scan with the
existing cached binding API. All 1,936 fresh original Ruby RSpec references match
and 49 focused native RSpec tests pass without fixture changes.

The combined framework-family gate covers 15,766 originals, 31,532 final states,
80 rules and 276 authored runtime scenarios. Its strict report retains 18,975
events. Twelve non-correctability warning observations are inherited documented
parser and stale-hash limits. Four all-mode runtime changes are inherited
explicit unsafe contracts, including one intentionally invalid original; this
is not a claim that all runtime programs preserve behavior. No invalid native
correction or non-idempotence category was reported. The core gate compares
3,621 of 3,627 sources across 98 rules, retaining six intentional
InterpolationCheck safety differences plus its declared exceptions/hazards.
No allowances were added to conceal these findings.

Actual CLI controls pass: 190 scope checks, 15 schema-cache checks, 32 physical
lockfile checks and four safety-configuration checks. Core's validation fix also
resolves the historical invalid-Safe configuration discrepancy.

Registry comparison preserves all 672 baseline names and adds the two previously
missing, target-inert rules. None of the 160 enabled baseline stub source paths
retains its original stub marker; this static observation is not a parity proof.
Fresh original config still enables 553/550 root/dev-spec names. Native config
lists 552/549, missing GraphQL/PrepareMethod: the locked plugin registers that
class without a default-YAML entry, exposing a versioned registration-inference
gap. A bounded fix and positive corpus coverage inventory remain pending.

The full Wistia scan exposed a panic in Style/TrailingCommaInArguments at line179
(reversed slice11417..11416). The core owner is preparing an isolated regression
fix. Cold/warm benchmark timings are invalid until that fix is integrated and
the full scan succeeds. Generic multi-cop correction scheduling remains deferred
at the user's request. Per-family reports retain their branch provenance;
this report separately records integrated validation and remaining limits.
