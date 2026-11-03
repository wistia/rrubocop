# Parity integration checkpoint

All seven frozen completed source snapshots are merged on
`feat/wistia-parity-integrated`. Exact snapshot SHAs and semantic merge decisions
are recorded in `wistia-parity-integration.json`; source branches remain preserved.
The latest validated code checkpoint is 091dc33: 1,086 native tests pass
with one ignored. The broad family differential below was run on c9d11a4;
subsequent registration and panic fixes have their own focused gates. Core's
tested numeric-span prerequisite is included; unfinished consumers are excluded.

One merge interaction required replacing RSpec's legacy numbered-block scan with the
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
Fresh original config still enables 553/550 root/dev-spec names. A bounded
exact-version registration supplement restores matching native 553/550 counts: the locked plugin registers GraphQL/PrepareMethod without a
default-YAML entry. Three actual CLI availability/disablement/scope controls
match Ruby.
All 160 former stubs now have mapped positive Ruby/native detection evidence
in wistia-baseline-stub-evidence.json. This is detection coverage, not a claim
that the documented correction or behavioral gaps are closed. The existing
GraphQL implementation still needs its full behavioral review.

The full Wistia scan exposed a panic in Style/TrailingCommaInArguments at line179
(reversed slice11417..11416). The isolated original-backed fix is integrated
as 091dc33: all 12 focused tests pass, both actual-file CLIs exit zero with no offenses, and the full
integrated suite passes 1,086 tests with one ignored. The full repository scan
and cold/warm benchmark still need to rerun; prior timings remain invalid. Generic multi-cop
correction scheduling remains deferred at the user's request. Per-family reports retain their branch provenance;
this report separately records integrated validation and remaining limits.

The next completed framework leaf checkpoint, 0b81f9d, adds TimesMap,
RefuteMethods and RequestReferer reviews without shared helper changes. Its
integrated gate covers 875 originals and 1,750 final states; all 51 valid authored
programs preserve behavior in 102 native observations, and six actual scope
controls match. The strict driver exits one, retaining 1,212 safety-contract
events (including 516 correctability-only diagnostics and 27 comparisons to
unsafe original runtime rewrites). No warning differences beyond correctability
remain. The combined full native rerun follows the next checkpoint.

DelegateAllowBlank and Exit add 504 originals and 1,008 final states, with
107 retained strict safety differences and no warning differences beyond
correctability. Seven valid programs preserve behavior in 14 native modes.
Two rescued invalid ActiveSupport API declarations are separate: one is an
intended option repair that changes output, and the false option is unmatched.
Four actual scope controls match. The full native checkpoint follows the
separately completed FloatOutOfRange leaf import.
