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

## Standard Style stubs and MixedCaseRange

The second batch implements three previously empty Style registrations:
`DateTime`, `DirEmpty`, and `RedundantCurrentDirectoryInPath`. It also implements
regexp detection and unsafe correction in `Lint/MixedCaseRange`, which previously
only inspected string range objects and missed the repository's actual regexp
finding.

Each corresponding `rubocop-1.65.1/lib/rubocop/cop/{style,lint}/<name>.rb` was
reviewed independently, followed by the unmodified original upstream spec at
`rubocop/rubocop@v1.65.1/spec/rubocop/cop/{style,lint}/<name>_spec.rb`. The original
spec results are 12 DateTime, 16 DirEmpty, 9 current-directory, and 30 mixed-case
examples, all passing in Ruby. Independent probes add namespace, safe-navigation,
block, comment, Unicode, decoded-character, numeric-radix, and escape-boundary
coverage. Current captured source/config replay counts are 26, 33, 19, and 52
respectively. They compare diagnostic messages, locations/highlight lengths and
one-pass corrected bytes. Already-corrected states are retained as negative
controls.

DateTime's replacements and MixedCaseRange's regexp replacements remain unsafe
(`-A` only by default), exactly as the original rule metadata specifies.
DateTime coercion offenses and MixedCaseRange string-range offenses are reported
without pretending to offer a correction. The class/date historical exception,
AllowCoercion, target Ruby minimum for DirEmpty, escaped range endpoints and
mixed/nested/interpolated regexp character sets are explicitly covered.

The source review additionally found two unsafe upstream corrections. These
are explicitly different safeguards, not hidden parity exceptions:

- `Dir.children( # keep\n path).empty?` is diagnosed by both engines. Ruby deletes
  the comment; Rust keeps the diagnosis but withholds that correction. Rust also
  preserves raw argument bytes using two surrounding edits rather than rebuilding
  arbitrary path arguments through UTF-8 strings.
- For `require_relative "\x2e/foo/./bar"`, Ruby's search for the first literal
  `./` targets a later path segment even though the leading `./` is encoded. Rust
  detects the same range but withholds a correction unless the decoded literal
  value proves that deleting that exact source span removes the intended prefix.

The actual Wistia `stats_helper.rb:470` regexp is now flagged with Ruby's same
message, warning severity, starting location and three-character highlight.
The fork deliberately does not emulate Ruby's mutation of shared regexp trees.

## TernaryParentheses

The third batch replaces the incomplete heuristic with the original
`rubocop-1.65.1/lib/rubocop/cop/style/ternary_parentheses.rb` algorithm, including
SafeAssignment and the original precedence/unsafe-removal branches. All 91
unmodified upstream Ruby specs pass; all 135 captured original/iterated cases
match Rust messages, locations, highlight lengths and correction bytes. The
compiled CLI passes 270 safe/all-mode byte checks and 270 idempotence checks.
The actual installer spec now produces Ruby's same two line-11 diagnostics at
columns 8 and 51, rather than silently missing both.

This includes the complex-condition style used by Wistia, simple/query/indexed
conditions, assignments, defined/yield, keyword operators, safe navigation,
multiline conditions, one-line pattern matching, and unsafe unparenthesized-call
correction refusals. A tree-sitter precedence difference for `not (bar) ? ...`
is normalized to Ruby's actual conditional span rather than rewriting `not`.

An independent source review found another upstream safety issue:
`(object&.predicate argument) ? a : b` becomes
`object&.predicate argument ? a : b` in Ruby autocorrection. Ruby's own parsed AST
shows the ternary moving from the outer expression into the safe-navigation
call's argument. Rust reports the offense but withholds this semantic-changing
removal. This is an explicit safety difference beyond the original passing
spec corpus, not a claimed exact match.
