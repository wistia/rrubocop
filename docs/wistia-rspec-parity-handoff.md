# RSpec parity handoff

Branch: `feat/wistia-parity-rspec`, pushed to the `wistia` fork remote.
Worktree: `/dev/shm/rrubocop-parity-rspec`.

All 96 owned enabled rules have entries in
[wistia-rspec-parity-progress.json](wistia-rspec-parity-progress.json): 80 original
no-op registrations implemented and 16 existing implementations reviewed against
pinned original Ruby source. The cumulative
[review](wistia-rspec-parity.md) records each coherent implementation batch.

The final corpus separates 1,936 strict original cases, 211 deliberate safeguards,
eight original-rule crashes and four invalid-original-syntax specimens. There are
53 correction-disable controls. Safe/all corrected bytes, exact offense messages
and locations, correction metadata, final offense state and idempotence are checked
per cop. Trusted runtime scripts/artifacts record concrete behavior and side-effect
controls; they do not execute arbitrary imported fixture code.

Original runtime: Ruby 3.4.9, RuboCop 1.65.1, rubocop-rspec 3.0.4,
parser 3.3.5.0, ast 2.4.2. Trusted RSpec probes use the Wistia bundle's
rspec-core 3.13.0 and rspec-expectations 3.13.2. Use `bin/rubocop-ruby` in
`/home/ubuntu/code/wistia-rrubocop-pr`; `bundle exec rubocop` invokes Rust there.

## Integration prerequisites

Prefer the complete branch history, or preserve its shared prerequisites when
cherry-picking individual rule batches:

- Existing cached local binding and static literal/symbol decoders in `ported`;
  shared integer/send-range helpers; typed string parts and delayed heredoc spans.
- Shared `ruby_expression::{same_expression,eql_expression,same_arguments,eql_arguments}`:
  ordinary AST equality for repeated bindings, strict equality for hash grouping.
- Canonical shared dump/inspect/symbol formatters, including the ASCII symbol-string
  inspection distinction and version-aware regexp decoding.
- `1e5bd7e`: empty quoted/%s symbols are dynamic AST shapes; `plain_symbol` preserves
  the distinction without changing runtime-value decoding.
- `9019831`: bare `__LINE__` is the original integer AST value at its source line;
  method-name tokens are excluded.

The framework agent imported the last two checkpoints and has additional original
AST pairs. Union `framework-expression-equality.json` during integration rather
than replacing its newer fixture. Its subsequent `plain_string` wrapper is outside
this branch's requirements.

Move/group corrections use existing correction ownership and identical-edit aliases
atomically. ReceiveMessages has a native regression checking correction credit for
both owning offenses. No generic scheduling engine was introduced.

## Explicit open limits

- The user deferred generic autocorrect incompatibility/scheduling. Original
  LetBeforeExamples/ScatteredLet incompatibility and combined-cop evidence remain
  documented; this work makes no combined-cop exact-parity claim.
- The import manifest retains 14 unresolved host-generated upstream templates.
  They are not counted as imported successful source cases.
- Global custom `RSpec: Language` aliases are not propagated by the native
  per-cop setup. Wistia's production config has none; the upstream custom-Includes
  host mutation is explicitly excluded from alias coverage.
- Deliberate safeguards preserve behavior where the broader original correction
  changes evaluation order, state, bindings, matcher truth semantics, callback
  order, continuation behavior, literal values, comments or disabled source.
  They are documented discrepancies, not strict correction successes.
- Shared AST comparison has bounded evidence. Broader unsupported AST/encoding
  families and `__FILE__`/`__ENCODING__` are not blanket equality claims. Stale
  precomputed upstream AST hashes are not reproduced.

## Reproducing checks

Build on tmpfs and avoid sharing another agent's target directory:

```sh
CARGO_TARGET_DIR=/dev/shm/rrubocop-rspec-target \
CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 \
PATH=/home/ubuntu/.cargo/bin:$PATH \
cargo test --locked --lib
```

The focused RSpec/RSpecRails gate contains 51 native tests; the shared expression
suite contains three tests and 71 original AST pairs on this branch. The final
complete native library gate passes 842 tests with one existing ignored test;
the non-test library check also passes.
