# Wistia core cop parity work

Reference implementation: Ruby RuboCop 1.65.1, as pinned by Wistia. The starting
inventory and reproductions are the comprehensive audit of fork a8c56ec. No
retirement of enforced rules or relaxed parity exceptions is authorized here.

## Layout/SpaceAroundOperators

The first batch ports the original operator and alignment branches instead of
rewriting the entire interval between operands. This closes comment/newline
loss, missing assignment/multiple-assignment detection, exponent/rational
spacing policy, line continuations, operator ranges/messages, and sibling
HashAlignment/ExtraSpacing configuration.

Original sources reviewed:

- `rubocop-1.65.1/lib/rubocop/cop/layout/space_around_operators.rb`
- `rubocop-1.65.1/lib/rubocop/cop/mixin/preceding_following_alignment.rb`
- `rubocop-ast-1.32.3/lib/rubocop/ast/token.rb`
- `rubocop/rubocop@v1.65.1/spec/rubocop/cop/layout/space_around_operators_spec.rb`

The unchanged original Ruby spec passes all 89 examples. Its 120 distinct
original/iterated source-and-config cases are replayed against the Rust cop,
plus 11 independently observed audit reproductions. Each replay compares
messages, starting locations, and one-pass corrected bytes; captures include
already-corrected states and negative controls. The Ruby examples cover
alignment enabled/disabled, adjacent assignments, operator tables, hash rockets,
ternaries, setters, inheritance, rescue bindings, exponent and rational styles,
Ruby-version-dependent pattern matching, and ForceEqualSignAlignment.

The independently reported comment deletion is retained as a negative regression:
`a+ # important note\n  b\n` must remain byte-identical and report no offense.
No safety exception is used to hide a difference. This bounded original-spec
coverage is not a proof of equivalence for every possible Ruby source.

## Remaining queue

The exhaustive owned inventory is recorded in
`scripts/parity_audit/core-gap-progress.json`. All rules outside RSpec,
RSpecRails, Rails, Migration, Performance, and FactoryBot belong to this queue.
Only rules with a completed implementation, original-source review, and parity
validation are marked completed; registration and clean scans are insufficient.

Validation for the operator batch: all 122 layout unit tests pass. The actual
compiled CLI also reproduces Ruby's corrected bytes for all 11 independent
regressions under both `-a` and `-A` (22 checks), with 22 unchanged second-pass
idempotence checks. Highlight lengths are compared for the captured original
spec cases as well as messages and starting locations. The source review and
131-case replay were performed separately before this batch was committed.
