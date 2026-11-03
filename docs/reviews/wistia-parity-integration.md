# Integrated parity verification

The combined source checkpoint is `362240f` on
`feat/wistia-parity-integrated`. Fresh original Ruby and optimized native runs
inspect the exact same **14,824 files** from Wistia
`310b8e42e91232d07210eff6cacc2f2035a01bda` and report the same **three offenses**.
There are no extra or missing diagnostics by cop, file, line, column, severity or
raw message. Both processes exit one for those findings, both stderr files are
empty, and the Wistia tracked working tree remains unchanged. Neither paired run
uses autocorrection. Only the native cop-name message prefix is normalized.

The full proof, provenance and complete-artifact hashes are recorded in
`docs/evidence/integrated-full-corpus-parity.json`. Complete stdout JSON, stderr,
file manifest, arguments and process statuses are retained at
`/tmp/rrubocop-full-paired-362240f`. The original engine is Ruby 3.4.9 with
RuboCop 1.65.1. The optimized binary's recorded source is 362240f; its version
header describes baseline defaults, while this project loads its pinned config.

Observed uncached times were 1,115.67 seconds for Ruby and 99.75 seconds for
native. A separate warm native run takes 0.47 seconds and returns exactly the
same complete JSON as the uncached native run. These paired observations are
separate from the controlled performance-only benchmark: the three commits
b7d9d9d, 14cc42d and 35fcdb8 reduced that frozen baseline from 390.66 to 98.24
seconds, with 0.45-second warm runs. Its deliberately preserved 169 findings
included known false positives and were never proof of Ruby agreement.

The initial real-corpus comparison exposed 166 Rust-only observations. They
were repaired across numbered parameters, regexp alternatives, parentheses,
RSpec block matchers, GraphQL option order, factory shorthand values and
numbered-block count candidates. The first repaired explicit-file run also
exposed a sibling nested-config discovery bug. Configuration now uses the common
directory for multiple operands and preserves single-file and explicit-config
behavior. Two order/scope regression tests pass. The fresh 74-file intermediate
gate and the complete 14,824-file gate both preserve the original three positives
and have no extra or missing diagnostics.

The integrated native suite passes **1,096 tests with one ignored**. Seventy
new original matcher cases agree across 210 native modes, including exact
messages, ranges, severity, correctability and source bytes. FloatOutOfRange's
730 public CLI modes match bytes, exits and scope, with 1,460 stable engine
second passes. Initial integrated scope/cache/config checks covered 190 path
controls, 15 schema-cache controls, 32 physical lockfile controls and four safety
configuration controls, all exact. The invalid-Safe CLI discrepancy is resolved.

Registry comparison preserves all 672 initial names and adds the two previously
missing target-inert rules. Fresh configured names match Ruby at 553 root and
550 dev-spec entries. Every one of the 160 initial enabled stubs has mapped
positive original/native detection evidence in
`wistia-baseline-stub-evidence.json`. Earlier broad differential runs, shared
merge decisions and exact source snapshots remain recorded in the JSON report.
Source branches are preserved; rollout PRs and S3 were untouched.

The verified scope is read-only diagnostics on this repository snapshot.
Autocorrection retains deliberate safeguards against unsafe original rewrites;
per-family parser, configuration and version limits remain documented. The
GraphQL optional string/symbol-style correction capability remains queued.
Generic combined-cop correction scheduling remains deferred at the user's
request. Later source reviews and the optional selector-routing optimization
are queued separately and are outside this validated checkpoint.
