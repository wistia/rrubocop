# Wistia core cop parity work

Reference implementation: Ruby RuboCop 1.65.1, as pinned by Wistia. The starting
inventory and reproductions are the comprehensive audit of fork a8c56ec. No
retirement of enforced rules or relaxed parity exceptions is authorized here.

## Current scope

Thirty-five owned rules have completed bounded original-Ruby validation,
including all thirteen original direct-file Style stubs, the additional
delegated RedundantParentheses stub and both absent version-gated cops.
The exhaustive 367-rule inventory retains 332 queued or in-progress entries;
some already have implementations. LiteralAsCondition is the current
implementation batch. Documented safety and syntax limits remain, including
additional UselessAssignment capture combinations. These bounded results do
not establish universal parity for arbitrary Ruby source. Earlier sections
retain their historical checkpoint counts; the newest completion section
and exhaustive ledger carry the current status.

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

## TrailingCommaInArrayLiteral and TrailingCommaInHashLiteral

The fourth batch ports the shared original `TrailingComma` mixin and
`PunctuationCorrector.swap_comma`, used by the pinned array/hash literal cops.
It adds the missing corrections, fixes inline literal detection, and matches
Ruby's messages and highlights on the last item or comma. Both delegates share
one literal implementation; existing argument-list behavior is unchanged.

All 33 original array specs and 28 original hash specs pass in Ruby. The Rust
matrix contains 62 array and 36 hash source/config cases, including 24 additional
original-Ruby branch probes for empty/commented literals, nested literals,
keyword hashes, splats, heredocs and percent-word arrays. All 98 cases match
messages, locations/highlights and corrected bytes. Both actual CLI modes pass
196 correction-byte comparisons plus 196 idempotence checks. The existing
12 trailing-comma tests and all 125 style tests pass.

All three literal policies (`no_comma`, `comma`, `consistent_comma`), single-item
inline closers, per-item line separation, comment trivia, heredoc markers and
last-line highlight selection are exercised. The pre-existing hash fixture's
message was corrected to the unchanged pinned Ruby rule's `item of a hash`
wording rather than preserving the old Rust placeholder wording.

## EachForSimpleLoop

The fifth batch implements this former Style stub from the complete pinned Ruby
source and its 20 unchanged upstream specs. All 32 captured original/independent
cases pass, including inclusive/exclusive ranges, nonzero/negative/radix bounds,
explicit block parameters, safe navigation and numbered parameters. Actual CLI
safe/all modes pass 64 correction comparisons and 64 idempotence checks.

Additional explicit safety guards retain diagnostics without rewriting when the
loop's Range return value is observed, when replacing the iterator would delete
comments, or when Ruby 3.4's implicit `it` would receive different values. For
example, the original Ruby rule rewrites `result = (1..5).each {}` into an Integer
result, and `(3..7).each { puts it }` into `5.times { puts it }`. A trusted authored
runtime probe confirms the latter changes `it` from `[3,4,5,6,7]` to `[0,1,2,3,4]`.
Those semantic changes are not copied into Rust.

A reverse-range concern was separately tested and disproven: Ruby's
`(5..1).each {}` and its `-3.times {}` correction both execute zero iterations.
This correction is retained and covered by the reference replay rather than
being incorrectly treated as an upstream hazard.

## MissingRespondToMissing

The sixth batch implements the former stub from RuboCop 1.65.1
`lib/rubocop/cop/style/missing_respond_to_missing.rb`. All eight unchanged
upstream specs and nine additional scope probes pass in the original Ruby
runner and the Rust replay (17 exact message/range cases). The native CLI
passes 34 safe/all-mode unchanged-byte checks and 34 idempotence checks.
The rule has no autocorrection.

The port preserves the original same-method-kind search, parent/grandparent
scope behavior, inline visibility modifiers and singleton receiver handling.
A file consisting only of `def method_missing; end` crashes the original
rule while accessing a missing parent; Rust reports the offense safely. This
reference crash is documented separately from the successful differential
cases and is not counted as exact parity.

## HashSyntax UTF-8 safety checkpoint

The existing symbol-name helper split the final byte of UTF-8 strings and
panicked on keys such as `:aé`. It now checks ASCII suffix bytes without
slicing a multibyte character. Twelve direct original-Ruby regex probes cover
non-ASCII tails, punctuation and the Kelvin-sign/long-S case folds accepted
by `/\A[_a-z]\w*[?!]?\z/i`. This is a narrowly bounded safety fix;
HashSyntax correction and broader configuration parity remain queued.

## HashSyntax delimiter port (shorthand branches remain queued)

Ports the delimiter algorithm and corrections from pinned Ruby 1.65.1
`lib/rubocop/cop/style/hash_syntax.rb`: all four delimiter policies,
`UseHashRocketsWithSymbolValues`, `PreferHashRocketsForNonAlnumEndingSymbols`,
quoted/interpolated keys, Ruby 2.1 versus 2.2+ gating, command spacing,
returned hashes and omitted values. Shared heuristic placeholders were removed.
Whitespace trimming follows the original RangeHelp sequence, preserving
indentation after a removed newline and retaining comments.

All 171 unchanged upstream specs pass in the original Ruby runner; this batch
ports the delimiter subset, represented by 147 original source/config states.
Another 142 states replay the delimiter sources with production Ruby 3.4 and
shorthand `either`. Independent edge probes add 23 exact cases (312 total),
including newlines/comments, UTF-8 symbols, omitted values and command syntax.
The Rust replay compares exact messages, positions, highlights and first-pass
corrected bytes. Ruby's Kelvin-sign case folding also exercises recovery of a
symbol tree-sitter otherwise misclassifies as an identifier.

A separate explicit safeguard fixture preserves the original diagnostics for
`return :a => 1, :b => 2` while producing one brace pair. The original rule
wraps once per flagged pair and produces `return {{a: 1, b: 2}}`, which Ruby's
compiler rejects. This reference syntax corruption is not copied. The fixture
retains both the original invalid bytes and the expected Rust correction.

This checkpoint does not implement the separate HashShorthandSyntax mixin's
`always`, `never`, `consistent`, or `either_consistent` branches for Ruby 3.1+.
Production Wistia uses `either`; broader shorthand parity remains explicit work
in the inventory, not a completion claim for the entire cop.

Validation for this checkpoint: all 131 Style tests pass; actual CLI safe/all
modes pass 624 exact correction-byte checks and 624 idempotence checks. The
returned-hash safety fixture additionally passes both CLI modes and their
second passes (two checks each), without claiming equality to invalid Ruby
output.

## YAMLLoad explicit version gate

Registers the previously absent Security/YAMLLoad implementation and ports
the complete pinned Ruby 1.65.1 `lib/rubocop/cop/security/yaml_load.rb`
algorithm. It explicitly returns for Ruby 3.1+, including Wistia's Ruby 3.4,
and checks root YAML/::YAML ordinary load calls on older targets. Namespaced
constants, safe navigation and other methods remain accepted. Only the
selector is replaced; unsafe autocorrection is retained as original metadata.

All six unchanged original Ruby specs pass (eight captured states). Forty
additional original-Ruby probes cover Ruby 2.7/3.0/3.1/3.4, omitted arguments,
qualified/namespaced receivers, safe navigation, explicit blocks, comments,
keyword options and colon dispatch. All 48 exact detection/correction replays
pass. Actual CLI safe/all modes pass 96 correction comparisons and 96
idempotence checks; safe mode leaves pre-3.1 calls unchanged. This gate is
represented in executable behavior rather than silently omitting the rule.

## NonDeterministicRequireOrder explicit version gate

Registers the previously absent rule from pinned Ruby 1.65.1
`lib/rubocop/cop/lint/non_deterministic_require_order.rb`, including the
explicit Ruby >=3.0 early return. Older targets cover Dir/::Dir indexing and
glob calls, direct blocks, each blocks, numbered parameters and
`&method(:require)` / `&method(:require_relative)` callbacks. Explicit method
receivers, safe navigation, other namespaces, invalid block shapes and
already sorted receivers remain accepted. Numbered parameter discovery
respects nested lexical scopes and method-name identifiers.

All 28 unchanged original specs pass in Ruby (40 captured states). Another
42 original-Ruby probes cover positive/negative matcher branches and the
production Ruby3.4 gate. All 82 exact message/range/correction replays pass;
actual safe/all CLI modes pass 164 correction comparisons and 164 idempotence
checks. The original unsafe policy is retained: safe mode reports without
reordering older-target sources.

Two independent original-Ruby hazard probes confirm that moving the block
pass deletes an intervening comment and that a callback-only glob crashes
while accessing the missing previous argument. Rust still reports those
inputs but withholds the corresponding correction; both safeguards have
dedicated regression tests. These cases are separate from the 82 exact
reference replays.

## RedundantArrayConstructor

The sixth formerly inert Style rule is implemented from the complete pinned
Ruby 1.65.1 `lib/rubocop/cop/style/redundant_array_constructor.rb`. All 13
unchanged upstream specs pass in Ruby; the matrix includes 37 exact original
source/config states after independent probes. Branches include root and
qualified Array.new, Array[], bare Array conversion, percent word/symbol
arrays, splats, nested element expressions, receiver chains, comments inside
retained arrays, and negative argument-count/namespace/safe-navigation cases.
All 134 Style tests pass. Actual CLI modes pass 74 correction comparisons
and 74 idempotence checks.

Independent original-Ruby probes confirm that an attached constructor block
is left on a bare literal after correction and that explicit `Array.[](1)`
becomes invalid `[](1)`; Ruby's compiler rejects both corrected forms. The
original also removes constructor comments outside the retained array. Rust
reports without correcting those inputs. It also refuses lossy conversion of
non-UTF-8 literal bytes. These explicit safeguards pass dedicated Rust tests
and eight actual CLI source-preservation/reporting checks. The no-argument
explicit `Array.[]` form remains safely correctable and is compared to Ruby.

Seven of the original 13 Style stubs remain: ComparableClamp, EachWithObject,
ExactRegexpMatch, RedundantRegexpArgument, RedundantRegexpConstructor,
NonNilCheck and TrivialAccessors. Remaining correction gaps such as
RedundantParentheses and UselessAssignment, plus HashSyntax shorthand branches,
remain queued in the exhaustive inventory.

## RedundantRegexpConstructor

The seventh of the original 13 Style stubs is ported from the complete
pinned Ruby 1.65.1 `lib/rubocop/cop/style/redundant_regexp_constructor.rb`.
All ten unchanged upstream specs pass in Ruby; 32 exact source/config states
cover root/qualified constructors, new/compile, percent delimiters,
interpolation, escaped delimiters, empty patterns, Unicode, canonical flag
ordering/deduplication and negative receiver/argument shapes. Exact Rust
replays and 64 native CLI correction comparisons plus 64 idempotence checks
pass. All 13 regexp-filtered regression tests passed before the additional
safeguard-only test was added; all three constructor tests then passed.

Independent original-Ruby compiler probes confirm invalid corrections for
`Regexp.new(%r{a/b})`, slash character classes, and attached blocks. The
original also deletes comments surrounding the retained regexp. Rust keeps
those diagnostics while withholding unsafe correction; four explicit
reference fixtures retain both original corrected bytes and the unchanged
Rust expectation, with exact diagnostic ranges/messages still compared.
The delimiter guard ignores slashes inside interpolated Ruby expressions,
retains existing escapes, and refuses non-UTF-8 replacement text. Twelve
actual CLI safeguard checks preserve source bytes and retain diagnostics.

Six original Style stubs remain: ComparableClamp, EachWithObject,
ExactRegexpMatch, RedundantRegexpArgument, NonNilCheck and TrivialAccessors.
The next priority is the broader RedundantParentheses correction/detection gap.

## RedundantParentheses

Ports pinned Ruby 1.65.1 `lib/rubocop/cop/style/redundant_parentheses.rb`,
its Parentheses mixin, and ParenthesesCorrector. The prior delegated helper
always returned false: this was an additional inert rule beyond the 13
direct-file Style stubs. All 203 unchanged original specs pass using the
original support/misc_helper.rb (`trailing_whitespace`); all 324 captured
states match exact messages, ranges and first-pass corrected bytes.

Covered branches include literals/numerics, variables and local scope,
constants, keyword arguments, unary/logical/comparison/method expressions,
interpolation, hash arguments, multiple expressions, ranges, post loops,
method-chain precedence, pin expressions, multiline control flow, orphaned
commas and heredoc comma relocation. Tree-sitter wrappers are adapted to
original Ruby parent/argument semantics. Pin-local resolution uses existing
model bindings when tree-sitter's expression-reference shape lacks an indexed
read. Sibling TernaryParentheses and ParenthesesAroundCondition settings are
passed through actual resolved configuration and exercised in eight native
production-version CLI checks.

All 139 Style tests pass. Native safe/all modes pass 648 exact correction
comparisons and 648 second-pass idempotence checks. Two additional actual
Wistia configuration/scope comparisons against bin/rubocop-ruby each verify
two positive offenses, exact messages/positions and corrected bytes.

One independently confirmed upstream safety issue is explicitly separate:
`[(1 # retain\n), 2]` is rewritten by Ruby into a comma joined onto the line
comment, and Ruby's compiler rejects that output. Rust preserves the original
diagnostic and withholds correction. An explicit reference fixture compares
its exact diagnostic while retaining both original invalid correction and
Rust unchanged bytes. Four native source-preservation/non-UTF-8 cases and
their second passes pass; ordinary raw-byte literals retain their bytes when
only parentheses are removed.

The capture helper now records the sibling settings and stores mutable
results on its module rather than in an autofrozen array constant. After
Ruby formatting/storage changes, re-running all 203 original specs produced
the identical 324 captured states. Running Wistia's full Ruby lint config on
this external standalone helper still reports its existing documentation,
metrics and Wistia-specific script/spec scope assumptions; none were disabled.

## Shared binding model checkpoint during UselessAssignment work

The original UselessAssignment matrix exposed three model defects: the
actual tree-sitter nested-target kind is `destructured_left_assignment`,
for-loop destructuring was not bound, and a module/class name receiver such
as `namespace::Nested` was not read in the outer scope. Those cases now
produce bindings/reads consistently with the pinned original Ruby specimens.
Two model regression tests exercise all nested names, their reads, and scope
isolation of module receivers. The full current native suite passes 862
tests with one unrelated existing ignored test; the only failing test is the
new in-progress UselessAssignment reference matrix (two named-capture gaps
and four explicit operator-declaration safety differences). This checkpoint
does not claim completion of that cop.

## UselessAssignment original-spec/dataflow checkpoint

Ports the pinned Ruby 1.65.1 UselessAssignment algorithm and its VariableForce
assignment/reference/branch behavior. Original sources inspected include
`lint/useless_assignment.rb`, `variable_force.rb`, and
`variable_force/{assignment,variable,reference,branch,branchable,scope}.rb`.
Name suggestions follow Ruby 3.4.9's DidYouMean spell-checker/Jaro-Winkler and
Levenshtein algorithm. Per-write liveness now handles overwritten writes,
branch exclusivity, incomplete rescue/ensure paths, modifier declarations,
loops/retry, block captures and method/scope boundaries. Corrections cover
plain, chained, sequential, multiple/rest, for, rescue, operator and named
regexp assignments, preserving the original unsafe autocorrection policy.

All 118 active original examples pass in Ruby; one additional upstream
example is already pending. Capturing those examples yields 176 states.
The upstream RSpec helper reuses a cop while creating a new Team for each
investigation, which retains ignored-node ranges across changed source in
chained-assignment examples. The committed refresh helper re-investigates
each state with a fresh original cop; native CLI investigation independently
confirms the fresh-source behavior. No stateful negative is used as proof
that a fresh source is accepted.

Rust matches 172 fresh states exactly on messages, ranges and first-pass
corrected bytes. Four explicit safeguard fixtures preserve the local
declaration in a fresh operator assignment, while comparing the identical
original diagnostic. An authored isolated runtime probe confirms the
original `foo ||= 1` value is 1 and its `foo || 1` correction raises NameError.
The rule's original documentation identifies this unsafe declaration change.
These four original corrected byte sequences remain visible in the fixture
and are not claimed as equal Rust corrections.

The full current native suite passes 864 tests, with one existing ignored
test. Across all 176 original sources in both real CLI modes, Ruby and Rust
match corrected bytes and exit codes for 348/352 comparisons. The only four
differences are the documented unsafe -A declaration removals; Ruby and Rust
are each byte-idempotent on all 352 unconditional second-pass comparisons.
There are no CLI errors. Two further real Ruby/Rust CLI comparisons preserve
non-UTF-8 RHS bytes without lossy string copying. The raw comparison driver
returns nonzero when any difference exists, including documented safeguards;
it does not allowlist them away.

The original-spec matrix is complete at this checkpoint. Additional
multi-capture, encoded-name and binder/reflection combinations remain queued
for separate original-source branch review before a broader parity claim.
No rule is retired or weakened to obtain these counts.

## Style/ComparableClamp

Reviewed the complete pinned `rubocop-1.65.1/lib/rubocop/cop/style/comparable_clamp.rb`, its `Alignment` indentation behavior, and all 23 unchanged original examples. All eight conditional comparison permutations, four nested array min/max patterns, the Ruby 2.4 gate, elsif indentation and absence of array autocorrection are represented in 25 captured original states. Eleven independent edge probes add explicit calls, normalized integer literals, empty statements, negative operators/branch shapes, attached blocks and three documented original correction hazards.

The separate parity review compares 36 exact diagnostic/message/range states. Thirty-three states also match corrected bytes; three keep the original offense but withhold correction: original raw-source bound inference can reverse otherwise equal AST sends (`x()` versus `x`), a statically reversed integer range raises in `clamp`, and an intervening comment would be deleted. The reference bytes remain recorded separately in `intentional-safeguards.json`. The real Ruby and native CLIs were each run twice in both `-a` and `-A`: 66/72 first-pass byte and exit comparisons match, with exactly those six deliberate differences, and all 144 second passes are stable. The comparison driver returns nonzero for these differences rather than masking them. Native replay also compares the three safeguards' exact original diagnostics. Four additional original/native production-config CLI modes under an actual `app/models` path produce the expected bytes and clean final states. The final integrated native suite passes 868 tests with one pre-existing ignored test.

Repeated NodePattern bindings use the shared original-AST `==` comparator, not source-text equality or the stricter grouping `eql?` relation. This imports the framework helper's 48 original expression pairs and preserves its documented conservative limits. Additional dynamic side-effect, encoding and unreviewed AST families remain unproven; this is a bounded original-spec port. Receiver precedence, invalid UTF-8 and delayed heredoc corrections are conservative guards, not claimed exhaustive semantic proof.

## Shared implicit numbered parameter classification

A separate scoped model regression follows the original pinned Ruby AST: `items.each { _1; _2 }` is a `numblock` with local-variable reads; a following top-level `_1` is a send; `-> { _1 }` is another `numblock`; `obj._1` and `_1()` remain sends. The native scope builder now predeclares actual `_1` through `_9` read identifiers only inside blocks without explicit parameters, stopping at nested scope boundaries. The cached local-identifier test checks the original seven identifier roles, including a nested block, rather than merely comparing two native caches. The integrated 868-test suite passes. This addresses the shared getter-classification gap; implicit Ruby 3.4 `it` and invalid explicit-parameter/numbered-parameter mixtures are outside this patch.

## Style/ExactRegexpMatch

Reviewed the complete pinned `rubocop-1.65.1/lib/rubocop/cop/style/exact_regexp_match.rb`, its unchanged 12 original examples, and `regexp_parser` 2.12.0 token behavior. Fourteen captured states plus 117 independent probes cover all five methods, safe navigation, escaped delimiters, the printable ASCII literal/metacharacter classes, intervals, options/interpolation negatives, Unicode plain literals, result context and capture state. The dedicated predicate follows the original first-three-token check: bos, an unquantified literal, eos. It preserves even the original diagnostic for an extra suffix after eos.

All 131 diagnostic/range/message states match. Corrected bytes match in 119 states; 12 retain the original offense while withholding unsafe edits. Recorded differences cover invalid single-quote replacement, unknown escape semantics, the ignored regex suffix, observed MatchData/index/case-equality/safe-navigation results, capture reads, an attached block, and deleted comments. Trusted authored Ruby probes confirm MatchData versus Boolean, stale capture state, String case equality versus string equality, and nil versus false differences. Negative `!~` retains ordinary correction when its Boolean result is observed and no capture is read. All original correction bytes remain in the explicit safeguard fixture; there is no parity allowlist.

Real original/native CLI validation initially covers 258 modes (236 equal bytes, 240 equal exit statuses, exactly 22 deliberate byte differences), with all 516 second passes stable; the final two result-context probes add four modes (two equal bytes/exits and two explicit case-equality safeguards), with all eight second passes stable. Combined: 262 modes, 238 equal bytes, 242 equal exits, 24 explicit safeguard differences, and 524 stable second passes. Four additional actual-production-config CLI modes under `app/models` produce the expected bytes and clean final states. The integrated native suite passes 870 tests, one pre-existing ignored. The separate native parity review then rechecks all 131 states after simplification.

This implements the original cop's bounded token predicate rather than a complete Regexp::Parser replacement. Escaped non-ASCII atoms, uncommon regexp escape/version branches, encoded non-UTF8 input and reflective capture access remain explicitly unproven. Comment and observed-result safeguards are conservative; no claim of universal Ruby semantics is made.

## Style/NonNilCheck

Reviewed the entire pinned `rubocop-1.65.1/lib/rubocop/cop/style/non_nil_check.rb` and `cop/ignored_node.rb`. All 21 unchanged original examples pass, producing 27 captured states. Seventy independent source/config probes bring the matrix to 97 exact diagnostic/range/message states. These cover both IncludeSemanticChanges branches, enabled/disabled comparison-style NilComparison siblings, exact ignored predicate result nodes (including endless methods and nested parentheses), explicit/implicit receivers, regular/safe sends, all three correction branches, unchanged substitutions, `not(...)`, and explicit bang/operator sends.

Ninety-two states match corrected bytes. Five retain the original diagnostics while withholding corrections that change arithmetic receiver precedence, remove text inside a quoted receiver, produce invalid explicit operator syntax, or leave an attached block on the replacement. The explicit safeguard fixture preserves the original generated bytes. The real original/native CLIs cover 194 modes: 184 equal byte pairs and 186 equal exit pairs, exactly ten intentional byte differences; all 388 second passes are stable. Four additional actual-production-config CLI modes under `app/models` produce expected bytes and clean final states. The final integrated native suite passes 872 tests, with one pre-existing ignored test.

The CLI harness now carries actual sibling-cop configurations alongside native injected options, so its original-Ruby comparison exercises the NilComparison gate rather than silently discarding it. The Ruby capture helper was simplified after its growing method exceeded ABC limits; `bundle exec rubocop -A` now reports no offenses for that helper. The 27 NonNilCheck and 14 ExactRegexpMatch captured original states were regenerated and compared unchanged after refactoring.

Unreviewed encoded inputs, additional comment/callback combinations, and exact correctable metadata for original no-op correctors remain bounded follow-ups; no claim of universal equivalence is made. All results in this report concern isolated cop execution. General cross-cop correction scheduling and incompatibilities remain explicitly deferred, and this batch makes no scheduling-engine changes.

## Shared regexp literal helpers

An isolated helper batch exposes `shared::is_literal_regexp_text`, the pinned original Util::LITERAL_REGEX repeated across a nonempty complete string. Its 545 original-Ruby predicate observations cover every ASCII byte, escaped forms and embedded forms, plus Unicode/Unicode whitespace negatives and escaped atoms. It also exposes the existing Ruby double-quoted escape decoder without changing its implementation, and extracts ExactRegexpMatch's capture-observation guard unchanged for independent consumers. The new predicate corpus and both existing ExactRegexpMatch matrices pass. The next RedundantRegexpArgument consumer remains a separate implementation batch.

The decoder models Ruby string escapes, including control/meta escapes. It must not be presented as identical to pinned StringInterpreter.interpret, which explicitly omits those control/meta branches. RedundantRegexpArgument also has its own quote/escape formatter; it is distinct from Util#to_string_literal and is not exported as a shared canonical formatter.

## Versioned regexp AST and source normalization follow-up

Imported the framework's pinned parser3.3.5.0 adapter (5186daa/696eb01) and added the discovered physical-line grouping branch: regexp escaped LF/CRLF continues one str child, while physical LF/CRLF starts another str child. c/C/m/M processing is versioned at Ruby3.1; standalone CR remains distinct. Context's existing escape decoder is reused. The static helper now preserves that single-str requirement, while regexp content joins remain separate. The original corpus now contains76 parser-version observations and60 AST equality pairs; all pass, as does the original545-row literal predicate corpus. Structural capture guards now recognize root-qualified, spaced and parenthesized Regexp constants; eight positive/negative receiver-role probes pass.

ExactRegexpMatch adds36 source/version cases, bringing its diagnostic/range/message matrix to167 states (155 exact correction states and12 prior safeguards). The new72 real CLI modes all match bytes and remain stable on all144 second passes. Sixteen exit comparisons expose the confirmed existing native global syntax-validation gap: Ruby reports Lint/Syntax for invalid UTF8 meta-escape regexps while native accepts their grammar. This is not an intentional correction safeguard or an allowlisted exception. Reproduction and original/native offense evidence are committed in `docs/evidence/regexp-invalid-meta-escape.json`; the generic comparison driver continues to report failure for these cases.

Ruby Parser::Source::Buffer normalizes CRLF in node source/ranges. Both regexp consumers now normalize diagnostic text and lengths accordingly. Empty Corrector.rewrite also normalizes source, but the original CLI leaves files unchanged when there are no edits; capture/refresh helpers therefore preserve raw source for empty correctors. All176 prior UselessAssignment reference states and14 original Exact capture states regenerate unchanged. All three edited Ruby helper files pass `bundle exec rubocop -A`. Cross-cop scheduling remains deferred.

## Style/RedundantRegexpArgument

Reviewed the complete pinned `rubocop-1.65.1/lib/rubocop/cop/style/redundant_regexp_argument.rb`, Util::LITERAL_REGEX, RegexpNode#content and the versioned parser lexer. All40 unchanged original examples pass, producing72 captured states. The expanded246-state original matrix covers all11 methods, regular/safe/implicit sends, first-argument/arity shapes, every printable ASCII literal/metacharacter class, percent delimiters, escapes/quotes/Unicode, empty and continued-empty regexps, physical/escaped LF/CRLF/CR, parser targets2.5/3.0/3.1/3.4, setter negatives and match-state reads.

All246 diagnostic/range/message states match. Corrected bytes match in239 states; seven preserve the original offense while withholding edits that change single-quote escape semantics, introduce string interpolation, trigger split's special whitespace behavior or change observed match state. Preferred formatting follows this cop's own escape whitelist and quote selection. The safety check decodes literal characters with existing helpers and parses the proposed Ruby string without executing fixture programs. Empty/multiline RegexpNode content joins remain distinct from ExactRegexpMatch's one-str NodePattern requirement.

Real original/native CLI checks cover492 modes:478 equal byte pairs,460 equal exit pairs,14 explicit safeguard byte differences, and18 additional exit-only differences from the separately recorded global invalid-regexp syntax gap. All984 second passes are stable; the comparison driver remains nonzero for differences. Four actual-production-config CLI modes under `app/models` produce expected bytes and clean final states. The integrated native suite passes877 tests, one pre-existing ignored. Trusted authored probes confirm partition capture-state and escaped-space split differences; a suspected byteindex offset difference was disproved (both reject the same middle-character offset), so no speculative guard was added.

Remaining limits include encoded non-UTF8 source, additional uncommon control/quote combinations and reflective/aliased match-state access. General syntax validation and cross-cop scheduling are not established by these isolated-rule results; the latter remains explicitly deferred.

## Cached variable roles: definition headers and reflective uses

Original Ruby AST probes confirm that implicit numbered parameters in singleton-method receivers, class superclasses and singleton-class receiver expressions belong to the enclosing block. The model predeclaration walk now includes those outer-evaluated headers while retaining ordinary method/body scope boundaries. Three header-role probes and the earlier numbered-parameter role regression pass.

A second independent role regression exposed synthetic usage bookkeeping: binding/bare-super mark variables as used at the call site, but those positions are not local-variable tokens. Read records now distinguish synthetic uses; cached membership filters them without changing usage counts or dataflow. Binding/local binding/binding() roles now match the original AST. Secondary shorthand bookkeeping and implicit reference helpers retain their usage semantics. The public model Read struct gains a synthetic field, and all in-tree constructors are updated. The integrated native suite passes882 tests, one pre-existing ignored; this includes the pending EachWithObject consumer that motivated the review.

## Style/EachWithObject

Reviewed the entire pinned `rubocop-1.65.1/lib/rubocop/cop/style/each_with_object.rb`, its block/numblock NodePatterns, basic-literal classification, assignment descendants and RangeHelp line-removal behavior. All13 unchanged original examples pass, yielding19 source/config states. Ninety-six independent completed original observations expand the matrix to115 exact diagnostic/range/message states:98 exact corrected states and17 explicit safeguards. One additional original investigation raises NoMethodError on a missing second parameter; native reports the style issue without crashing or changing source.

Coverage includes explicit/optional/rest/destructured/shadow/keyword parameters; basic versus composite initial literals, pseudo literals and heredocs; keyword argument grouping; assignments/for/destructuring; pattern binds and pins; returned-parenthesis shapes; numbered arity/version/definition headers; comments/line removal; reflection and next control flow. Pattern binding, non-memo next values, splat/unsupported parameter roles, reflection, nested numbered scopes and shorthand keys can change behavior under the original conversion, so their original diagnostics remain with no correction. Source formatting was split into pattern/safety helpers after review; no new lexical decoder was introduced.

Trusted authored Ruby programs confirm concrete original conversion differences: next nil returns nil versus the original object, pattern rebinding returns the element versus the original object, first-rest/shadow parameters alter returned values, local_variables ordering changes, and numbered shorthand can capture the memo recursively instead of the element. These are recorded separately from ordinary parity. The one-param original failure is preserved as original-failures.json, not treated as a matching Ruby result.

Real original/native CLI validation covers230 modes:196 equal bytes,198 equal exits and exactly34 intentional safeguard byte differences. All460 second passes are stable. Four actual-production-config modes under `app/models` produce expected bytes and clean final states. The full native suite passes882 tests, one pre-existing ignored. Unreviewed encoded/CRLF whole-file rewrites, additional scope/parameter forms, overrides and reflective access remain bounded follow-ups; cross-cop scheduling remains deferred.

The original EachWithObject one-parameter failure was also replayed through both real CLI modes: bytes and exits match, but Ruby emits an internal cop error while native emits an uncorrectable offense. The comparison harness now retains stderr and detects that original cop-error path even when exit1 is otherwise valid; both modes correctly remain reported as differences.

## Layout signed-command follow-up

A separate14-mode original/native CLI probe confirmed a source-semantics gap beyond the bounded SpaceAroundOperators corpus: Ruby keeps `puts +1` as a command unary argument, but native currently inserts a space and changes parsing to addition. Nested `foo +1 +2` and receiver calls are also affected; declared-local and explicit-parentheses controls distinguish real addition. The framework agent owns the shared semantic-command adapter and core owns Layout integration. This was confirmed beyond the earlier131 original-spec comparisons and is addressed by the operator-role follow-up below.

## Layout signed-command operator-role fix

Imported framework1ad6ac3's verified `shared::command_unary_plus` role adapter and made SpaceAroundOperators skip those unary signs while retaining genuine binary operators. The adapter is deliberately an operator-role projection, not a full command-argument AST. All318 original lexer-role observations regenerate unchanged across target Ruby2.5/3.0/3.4; focused native checks and Ruby helper lint pass. The Layout corpus now replays138 exact diagnostic/message/range/correction states.

The previously failing14 real CLI modes now match original bytes and exits completely, and all28 original/native second passes remain stable. The regression includes prints, nested command operators, explicit parentheses, receiver calls and declared-local controls. Eight actual-production-config modes under app/models and eight final-state checks also pass. The integrated native suite passes883 tests, one pre-existing ignored. Full command-call argument grouping remains a separate framework adapter task; cross-cop scheduling is still deferred.


## Style/TrivialAccessors

Reviewed the full pinned `rubocop-1.65.1/lib/rubocop/cop/style/trivial_accessors.rb`, AllowedMethods defaults, MethodIdentifierPredicates assignment-method classification, and the unchanged upstream spec. All37 original examples pass. Independent scope/configuration investigations expand the captured corpus to166 exact diagnostic/message/range states:155 exact corrected states and11 explicitly withheld unsafe or insufficiently proven corrections.

Branches cover readers/writers and argument roles, ExactNameMatch, AllowedMethods overrides and initialize, AllowPredicates, AllowDSLWriters, IgnoreClassMethods, arbitrary/self singleton receivers, nearest class/module/eigenclass scope, ordinary versus numbered instance_eval blocks, top-level method grouping, visibility calls, endless methods, Unicode, comments and observed definition values. The previously verified numbered-parameter traversal is shared with EachWithObject rather than duplicated; that cop's115 original states remain unchanged.

The safeguards retain offenses for comments, observed method-definition results, predicate singleton readers, nested methods, instance_exec, numbered instance_eval and conservative unknown-block contexts. Trusted authored Ruby probes confirm that the predicate correction raises NameError, replacing an observed def result changes Symbol to Array, and instance_exec replacement raises NoMethodError. These are separate intentional differences rather than parity successes. Arbitrary dynamic attribute-method overrides, encoded source and additional reflective contexts remain unproven.

Real original/native CLI validation covers332 safe/aggressive modes:310 equal corrected-byte pairs,312 equal exit pairs, exactly22 documented safeguard byte differences and no internal cop errors. All664 original/native second passes remain stable. Four actual Wistia-config app/models modes produce expected bytes and clean final states. The integrated native suite passes885 tests, one pre-existing ignored. A separate final source/branch review and unchanged37-example original-spec rerun passed before checkpointing; these are isolated-rule results, with generic correction scheduling still deferred.

## Style/HashSyntax shorthand branches

The shorthand follow-up ports the complete pinned `rubocop-1.65.1/lib/rubocop/cop/mixin/hash_shorthand_syntax.rb` decision tree alongside the earlier delimiter port. Reviewed all five styles, Ruby3.1 gating, per-hash omitted/omittable/needed grouping, predicate and non-symbol exclusions, modifier-form calls, bracket receivers, last-expression/assignment traversal and command-parenthesis insertion. All171 unchanged original HashSyntax examples pass; their252 source/config states and201 independently observed states now cover453 exact diagnostic/message/range cases. Of those,447 match first corrected bytes and six retain offenses while preserving source.

Independent coverage includes local/send values, constants, pseudo literals, Unicode and quoted keys, explicit receiver/argument calls, keyword splats, implicit-array hashes, brackets, return/break/super/yield, blocks and block passes, nested commands, multiple assignments and already-corrected controls. Four safeguards prevent an original unmatched opening parenthesis when the final hash pair has no offense; static parsing of the original correction confirms invalid Ruby. Two safeguards preserve comments deleted by original whole-pair replacement. The same cop still handles delimiters; no enforcement option was relaxed. Source/config replays and the implementation's branch review were performed separately.

Real original/native CLI validation covers906 safe/aggressive modes:894 equal corrected-byte pairs,902 equal exits and exactly12 byte differences for the six recorded safeguards. No internal cop errors occurred; all1812 second passes are stable. This includes846 main modes and60 supplemental implicit-array/splat/sibling-comment modes after their scope follow-up. Eight actual Wistia-config modes confirm that the active either setting preserves explicit values while hash rockets still convert; all final states are clean. The expanded native suite passes889 tests, one pre-existing ignored. Additional encoded-source, uncommon grammar and combined-cop interactions remain unproven; correction scheduling remains deferred.


## Layout/LeadingEmptyLines

Reviewed the pinned original cop and parser3.3.5.0's lexer/literal.rb monolithic-string, newline-flush and block-comment token emission branches. All10 unchanged upstream examples pass. The implementation now locates the first visible token and reports its exact message/location/length, leaves empty or whitespace-only sources unchanged, removes indentation before the first token on later lines, and ignores data-only __END__ input.

The60-state original replay covers comments/shebangs/block comments, trailing EOF comment ranges, ordinary/interpolated/percent/command strings, physical versus escaped LF/CRLF, numeric/rational/imaginary tokens, symbols, heredocs, Unicode/BOM, continuations and semicolons. All detection and corrected-byte states match with no declared safeguards. CRLF normalization occurs only when a correction is made, matching the original source buffer; empty/no-offense source remains byte-identical. Separate final algorithm review and original-Ruby replay passed.

Real CLI validation covers120 safe/aggressive modes, all matching original bytes and exits; all240 second passes are stable. Four production-config app/models modes and final clean states pass. Full native validation passes890 tests, one pre-existing ignored. Encoded source, additional parser token families and code-plus-DATA CRLF behavior are not established by this bounded corpus; generic correction scheduling remains deferred.


## Style/NilComparison

Reviewed the entire pinned original cop, its two exact send NodePatterns, supported styles/defaults, first textual substitution and parent-negation wrapping. All six unchanged original examples pass, yielding10 original source/config states. Independent source investigations add135 states, including26 original AST observations for signed numeric receivers. The145-state replay matches every diagnostic/message/selector range;129 first corrected states match exactly and16 explicitly preserve source for unsafe or insufficiently proven conversions. Existing fixture pointers were corrected to the original selector positions.

Coverage includes both styles, equality/triple equality versus negative controls, nil on either side, explicit operator sends, safe navigation, argument/block shapes, whitespace/comments/continuations, negation and parentheses, chains/indexing/operators, assignment/logical/ternary contexts, Unicode/CRLF, source literals and defined? observations. Signed numeric prefixes preserve Ruby's folded receiver semantics while ordinary unary predicate results retain safeguards. Supported no-op substitutions preserve source; original negation wrapping and CRLF buffer normalization still occur where applicable. The implementation edits exact spans and never decodes/rebuilds arbitrary receiver bytes.

The16 safeguards cover invalid Ruby generated around continuations, explicit operators, empty-call parentheses and blocks; precedence/receiver changes around chains, indexes and unary/binary parents; defined? observation and first-match substitution inside receiver literals. Static parsing records invalid original corrections. Trusted authored Ruby probes confirm changed receiver arguments, String versus FalseClass chain results, changed unary failure behavior and BasicObject method-availability observations. No fixture programs were executed.

Real CLI validation covers290 modes:258 equal corrected-byte pairs,258 equal exits and32 byte differences exactly corresponding to those16 safeguards. Ten additional exit-only differences are original infinite-loop failures on no-op corrections (exit2 versus native uncorrectable offense/exit1). All580 second passes are byte-stable, including those original failure states; stability does not turn the failing Ruby runs into passes. The driver now recognizes the original loop diagnostic and replays those10 modes separately without an allowlist. One further nested-predicate source triggers original Parser::ClobberingError and writes invalid code in both CLI modes; native reports both offenses and preserves valid source. Its nondestructive original diagnostics and failure evidence are recorded separately. No scheduling-engine changes were made.

Four actual-production-config modes and final clean states pass; the full native suite passes894 tests, one pre-existing ignored. Additional command-unary receiver forms, encoded source, code-plus-DATA normalization and unreviewed dynamic/reflective method behavior remain bounded follow-ups; generic scheduling and syntax expansion remain deferred.


## Layout/LeadingCommentSpace

Reviewed the complete pinned original cop's regexp, first-line exceptions, Doxygen and Gemfile option branches. All21 unchanged original examples pass and26 path-aware source/config states regenerate identically. The additional112 original states give138 exact diagnostic/message/full-comment range and corrected-byte matches, with no declared safeguards in the pinned supported-option domain.

The port fixes the incorrect one-character highlight and acceptance of consecutive shebangs after line1, and uses Ruby's ASCII whitespace set including vertical tab. Coverage includes hash runs/toggles/equals, inline comments, Unicode, whitespace, CRLF, magic comments, string/heredoc/block-comment negatives, data-only input and all four supported option combinations. Gemfile/config.ru and nested-name cases use their actual fixture paths. Existing opt-in native RBS/Steep annotation settings, absent from pinned1.65.1 and unused by Wistia, remain separately tested native extensions; parity is not claimed for enabling those unsupported reference options.

The original capture helper now records paths and retains distinct path cases during deduplication; the Rust replay honors them. The real CLI comparator creates relative fixture paths inside its temporary root and checks both engines' first/final inspected-file counts, avoiding a filename-only false pass. Unsupported absolute/escaping paths fail explicitly instead of silently changing the specimen. Existing fixtures without paths retain the prior default. The Ruby helper passes the Wistia Ruby3.4 lint configuration, and its style-only normalization leaves all26 original observations unchanged.

Real CLI validation covers276 safe/aggressive modes: all276 match bytes, exits and inspected-file counts, with one inspected file per engine in every mode. All552 second passes are stable. Four actual-production-config modes and final clean states pass; the complete native suite passes896 tests, one pre-existing ignored. Additional encoded/BOM source, code-plus-DATA normalization and unreviewed comment/token families remain bounded follow-ups; correction scheduling remains deferred.

## Explicit reference-version pinning

Lockfile-free synthetic CLI configs previously selected the native1.91 baseline while receiving explicit cop options captured from Ruby1.65. Their reported byte comparisons remain measured bounded observations; they do not establish parity for version-sensitive branches. Actual Wistia-config probes used its1.65 lockfile. The capture helper now records the original software version separately from TargetRubyVersion, the Rust original-spec replay injects that version, and the synthetic CLI runner passes native --rubocop-version explicitly (default1.65.1). Newly captured hash-brace observations regenerate identically, and its version-sensitive pattern controls are covered by the completed hash-brace gate below.


## Layout/SpaceInsideHashLiteralBraces

Reviewed the complete pinned original cop, SurroundingSpace and rubocop-ast1.32.3 Token whitespace/type behavior. All28 unchanged upstream examples pass, yielding40 source/config states. The216 independent observations cover all three styles and both empty-brace settings, nested/compact delimiters, comments, implicit keyword hashes, Ruby1.65 pattern exclusions, Unicode, whitespace controls, heredocs, regexps and static/interpolated/percent strings. All256 direct cop diagnostic/message/range and first corrected-byte states match, with no declared safeguards.

The port implements empty and multiline-whitespace hash correction, compact adjacent braces, original spacer ranges/messages and independent opening/closing comment handling. A bounded lexical-boundary helper reuses LeadingEmptyLines' verified monolithic-string/numeric handling, retaining its60-state replay. Literal delimiter types keep percent-string closing braces distinct from Ruby hash/block braces. Normalization edits carry an explicit owning offense in the cop; no scheduling or engine policy was changed.

The initial explicitly pinned1.65 CLI gate covers512 modes with matching exits/scopes and498 matching bytes; its14 CRLF failures exposed unowned ancillary edits being removed by the real pipeline. After the cop-local fix, all36 CRLF modes were rechecked and match bytes/exits/scopes. Together with the unchanged non-CRLF branch, the composed512-mode result has zero remaining differences and1024 unique stable second passes. A real-engine CI regression checks both safe/aggressive modes and clean second passes for single/nested CRLF hashes. Eight Wistia-config modes/final states pass; the full native suite passes899 tests, one pre-existing ignored.

Direct cop ranges match original zero-length control-whitespace ranges; the existing engine formatter enriches zero highlights to one, so identical rendered/wire range metadata is not claimed. Additional token families, encoded/BOM source, code-plus-DATA normalization, directive interactions and future-version pattern behavior remain bounded follow-ups. Correction scheduling and generic syntax expansion remain deferred.


## Omitted-value role evidence: underscore labels

An initially proposed local-cache fix was rejected before commit by original-source validation. Parser3.3.5.0 builders/default.rb#pair_label classifies a label with a lowercase first character through ident/accessibility; other names become CONST. Consequently `_value=1; {_value:}` is CONST/pure in the pinned AST, despite a runtime local binding. The existing local-identifier cache correctly remains false for that value; no model syntax sites or usage reads were added.

The45-case reference records48 roles (44CONST,2LVAR including Unicode lowercase,2SEND controls), with software/parser/AST/runtime versions, byte offsets and purity. Scope/declaration/parameter/header/numbered/Unicode controls pass through the actual cached-role API, and ignored underscore usage counts remain unchanged. The generator passes the Wistia Ruby3.4 lint configuration and regenerates identically. Full native validation passes901 tests, one pre-existing ignored. This checks local membership, not complete metrics/vcall classification or purity equivalence.

A separate trusted authored Ruby3.4 probe proves that an unbound underscore label can execute a method, while a bound underscore label reads its local. Canonical AST role and correction safety therefore remain distinct. The framework agent owns the shared Constant/Local/Send omitted-value adapter for its consumers; this evidence commit makes no production model change and does not mark another cop complete.


The subsequent model-only correction moves the existing underscore-label return before unresolved-call recording. Local membership and ignored usage counts stay unchanged, while all48 captured value sites now also agree with original CONST/LVAR/SEND call-site classification. Full native validation still passes901 tests, one pre-existing ignored. No fields, constructors or binding identities changed. ABC's current identifier-only dispatch means this metadata change is not presented as a demonstrated metric-score fix; other non-lowercase label classes and shorthand ABC scoring remain separate queue items.


## Layout/HashAlignment source-review checkpoint (in progress)

The pinned1.65.1 original cop, HashAlignmentStyles mixin and AST hash/pair geometry have been reviewed. All119 unchanged upstream examples pass in Ruby, producing167 source/configuration observations. The capture helper now preserves the actual ProcessedSource Ruby version and the sibling ArgumentAlignment configuration, and tolerates upstream examples with deliberately minimal configuration hashes. Its Wistia Ruby3.4 lint check passes.

This checkpoint contains the original observations, not a completed native port. The pending implementation includes separate rocket/colon styles and style arrays, separator/table geometry, keyword splats, implicit hashes, last-argument and fixed-argument exclusions, full ranges and guarded multi-edit correction. Native detection/correction parity, original invalid-style behavior, real CLI final states and idempotence remain unvalidated for this rule.


## Layout/HashAlignment implementation checkpoint (CLI gate pending)

The native port now follows RuboCop1.65.1 layout/hash_alignment.rb and mixin/hash_alignment_styles.rb, plus rubocop-ast1.32.3 hash/pair/keyword-splat geometry and Util#begins_its_line?. All119 unchanged original examples pass and their167 source/configuration observations regenerate identically. An additional293 independent valid states cover separate rocket/colon styles and style arrays, keyword splats, omitted values, Unicode, multiline keys/values, inline pairs, nested hashes, implicit call/array/index hashes, setters, super/yield/return, all last-argument policies, sibling ArgumentAlignment flags and24 CRLF cases.

All460 diagnostic/message/full-element range observations match. First corrected bytes match456 states; the remaining4 retain the original offenses while declining original corrections that delete source characters or cross a newline. Actual original malformed outputs are preserved separately in safeguards.json rather than overwritten. Negative adjustments are applied only to spaces/tabs; the whole element's correction is declined if an adjustment is unsafe. CRLF normalization has an explicit owning offense and runs only when this cop actually emits an edit. No generic correction scheduling policy changed.

The original alignment-selection algorithm is preserved, including separate delimiter instances, persistent Table widths, duplicate first-pair entries when choosing the shortest offense group, insertion-order ties, and correction deltas from the first configured style rather than the style selected for the message. Cop code is split into geometry, syntax projection and guarded correction modules. Superseded HashAlignment-only shared wrappers were removed; ArgumentAlignment's shared implementation is retained.

The complete native suite passes904 tests, one pre-existing ignored. Eight actual Wistia-config safe/aggressive engine modes produce matching bytes and clean stable final states. The full pinned real-CLI matrix is still running; HashAlignment is not marked complete yet. Invalid-style configuration, additional parser/encoding/token families and combined-cop scheduling remain outside this bounded gate. The legacy native EnforcedStyle alias is retained but is not claimed as a pinned original option.

Seven configuration states using a comment followed by a newline before => are invalid in the original parser; their error diagnostics and native detection discrepancy are recorded in docs/evidence/hash-alignment-invalid-grammar.json. They are a deferred generic syntax gap, not valid-source cop negatives or intentional correction safeguards.

## Additional queued original correction-safety evidence

The Performance agent observed original Lint/InterpolationCheck activating interpolation in an intentional literal template while linting scripts/framework_parity/refresh_expression_properties.rb (baseline92b860a, corrected55c78f1); PercentLiteralDelimiters and MethodCallWithoutArgsParentheses then changed the newly active expression. Its trusted generator now uses String#dump and round-trip checks. This is queued InterpolationCheck safety-review evidence, not a proven StringLiterals defect or a completed cop port.


### HashAlignment final bounded gate

The pinned real CLI matrix completed920 safe/aggressive modes:912 corrected-byte matches,920 matching exits and inspected-file counts,1,840 stable second passes and zero cop errors. The eight byte differences are exactly the four already documented source-loss safeguards in both modes. Both engines end with866 clean modes and54 modes retaining offenses; these totals are not presented as an all-clean result.

A separate fresh-original replay validates370 unique final source/configuration states: all diagnostics/messages/full ranges and bytes match, all parse successfully in the original, and none emits another edit. Seventeen unique states retain original offenses with an empty corrector. The safeguard specimens deliberately retain their original native offenses and source instead of the original malformed output. Eight actual Wistia-config modes remain clean and stable. The native full-suite checkpoint is904 passing tests/one ignored; the additional final-state replay passes independently.

The separately reviewed bounded HashAlignment port is now complete in the ledger, bringing owned bounded ports to28;13/13 direct Style stubs remain implemented.339 owned rules still await their full original-source/parity gate, including ArrayAlignment now in progress. This count does not claim exhaustive cop, wire-format, future-version, cross-file cached-style, encoding, directives or combined-cop parity. The documented invalid grammar and generic scheduling work remain deferred.


## Layout/ArrayAlignment implementation checkpoint (CLI gate pending)

Reviewed the complete pinned1.65.1 array_alignment.rb, Alignment mixin and AlignmentCorrector, plus AST array source/bracketing behavior. All25 unchanged upstream examples pass and yield27 original states. The native port adds original messages/default style, first-element checking, fixed indentation width, nested multiline correction and overlap deferral, implicit assignment arrays, rescue exception arrays and percent-array elements. Implicit hash pairs are grouped as one Ruby array element; parallel assignment is excluded.

All227 direct diagnostic/message/full-element range states pass:209 exact first corrections and18 documented safeguards. Independent cases cover both styles and widths2/4, Unicode prefixes, comments/blank lines/tabs, nested arrays, implicit arrays/hashes, splats, return/break/next controls, multiline expressions/strings/commands, interpolation, heredocs, document comments and32 CRLF states. String/heredoc protection is restricted to descendants of the corrected element, preserving code correction inside an enclosing string interpolation.

A separate original-AST review proves18 original corrections change multiline regexp bodies, symbol values or continued percent-word array values. The generic native correction protects those literal interiors. Actual original corrected bytes and before/after literal values remain recorded; all18 native expected outputs preserve the original AST literal values. No fixture programs are executed. These are intentional safety differences rather than hidden matching exceptions.

Display geometry reproduces the pinned unicode-display_width2.5.0/Unicode15.1 default API with generated scalar intervals and license attribution in NOTICE, without another runtime dependency. An independent serialized original table verifies all1,112,064 valid Unicode scalars;19 original whole-string observations check combining/wide/emoji/control behavior and negative-sum clamping. The generator rejects a different reference gem version.

The native suite passes911 tests, one pre-existing ignored. A real-engine regression checks both safe/aggressive modes for CRLF and nested multiline corrections, preserving ownership and clean stable second passes. Ruby helpers pass the actual Wistia configuration's lint correction. Real pinned CLI final-state and production-scope gates are still pending; ArrayAlignment is not marked complete. Additional encodings, source families, original future-version behavior, directives and combined-cop scheduling remain outside this bounded checkpoint.


### ArrayAlignment final bounded gate

Sixteen additional original tab/removal/nested/blank-line states validate the corrector's distinct space/tab range branches. The complete243-state first-pass gate has matching diagnostics/messages/full ranges,225 exact corrections and18 literal-preservation safeguards. The27 upstream states and pinned Unicode reference regenerate identically.

The pinned real CLI matrix completed486 safe/aggressive modes:450 byte matches,486 matching exits/scopes,972 stable second passes and zero cop errors. The36 byte differences are exactly the18 reviewed literal safeguards in both modes, and every native CLI result matches its reviewed safe output. Each engine finishes476 modes clean and10 with offenses; this is not an all-clean result.

A fresh original replay validates184 unique final native source/configuration states, including the protected literal outputs. All diagnostics/messages/ranges and bytes match, all parse successfully, and none emits another correction;5 unique states retain original no-op offenses. Nine additional detection/correction controls (18 engine observations) match bytes, exits and corrected flags for detection-only, AutoCorrect disabled/false and Safe/SafeAutoCorrect overrides. The committed control runner executes only the linters, never specimen programs. Eight actual Wistia-config modes/final states pass. The complete native checkpoint is911 passing tests/one ignored, followed by the focused final-state and tab replays.

The separately reviewed bounded ArrayAlignment port is complete. Owned bounded ports now total29 with13/13 direct Style stubs implemented;338 owned rules still await their full original-source/parity gate. Additional source/version/encoding/directive/wire-format families and combined-cop scheduling remain outside this bounded claim. ParameterAlignment source review is next; no implementation is claimed for it yet.


## Layout/ParameterAlignment implementation checkpoint (CLI gate pending)

Reviewed pinned1.65.1 parameter_alignment.rb and its Alignment/AlignmentCorrector paths. All19 unchanged upstream examples pass, yielding22 states. The native port restricts inspection to ordinary/singleton method definitions and implements both original messages, first-parameter/default geometry, configured fixed indentation and whole-parameter multiline correction. Shared layout behavior was extracted from the validated ArrayAlignment port into original_alignment; all affected Array/Hash/alignment family replays continue passing. This is a cop helper extraction, with no generic correction-engine or scheduling-policy changes.

All186 first-pass diagnostic/message/full-range states pass:174 exact corrected-byte states and12 explicit safeguards. Independent coverage includes bare/parenthesized/endless/singleton/Unicode-bearing definitions, keyword/optional/destructured/rest/block/anonymous/forwarded parameters, method/block/lambda scope controls, comments/blank lines/tabs/continuations, multiline defaults, strings/heredocs/interpolation, both styles and widths2/4, and32 CRLF states. Separate original AST observations prove all12 original regexp/symbol/percent-word corrections change literal values while the native expected outputs preserve them.

The full native suite passes916 tests, one pre-existing ignored. The shared width table and all ArrayAlignment243 first-pass/184 final states remain covered, including real-engine correction ownership. The original-literal and metadata-control runners now use generic alignment names to support both consumers. Ruby helper lint checks pass. Real CLI, correction controls, production scope and fresh final-state gates are pending; ParameterAlignment is not marked complete.

This review also found an untested configuration branch in the preceding Array gate: the original Alignment mixin falls back from a missing/null per-cop IndentationWidth to Layout/IndentationWidth, then2. The preceding synthetic matrices specified per-cop widths explicitly. Native config resolution now supplies the fallback for ArrayAlignment and ParameterAlignment while preserving explicit numeric widths, including0;12 resolution/precedence assertions pass. Existing ComparableClamp and body-rule resolution semantics are preserved. Real CLI fallback controls are pending separately.

Four singleton-method-on-literal source/config states are rejected by the original parser and accepted by native grammar. They remain documented in docs/evidence/parameter-alignment-invalid-singleton-literal.json as deferred generic syntax behavior; valid Unicode-bearing receiver-call replacements are used in the cop corpus. Additional source/version/encoding/directive/wire-format families and combined-cop scheduling remain outside this bounded checkpoint.


### Alignment indentation-width fallback follow-up

The supplemental original CLI gate now passes12 safe/aggressive cases for each of ArrayAlignment and ParameterAlignment (48 engine observations total). Missing/null own widths, explicit own2/0, global4/0 and null-global fallback2 all match bytes, exits and corrected flags. This closes the previously untested global-width branch for both consumers; no Wistia configuration or generic scheduling behavior changed.

ParameterAlignment's9 detection/correction metadata controls (18 engine observations) and8 actual Wistia-config engine/mode probes also pass. Its broader CLI/fresh-final-state gates are still running.


### ParameterAlignment final bounded gate

The pinned real CLI matrix completed372 safe/aggressive modes:348 byte matches and exactly24 reviewed literal-preservation safeguards, with all372 exits/scopes matching,744 stable second passes and zero cop errors. Every final mode is clean; every guarded native CLI output matches its reviewed safe output. The original19 examples/22 states regenerate identically.

A fresh original replay validates145 unique final native source/configuration states, including the protected literal outputs: all diagnostics/messages/ranges and bytes match, all parse successfully, no offenses remain and no further correction is emitted. Nine metadata/no-correction controls,12 width-fallback/precedence modes and8 actual Wistia-config engine/mode probes pass. The full native checkpoint remains916 passing tests/one ignored, followed by the additional focused final-state replay.

The separately reviewed bounded ParameterAlignment port is complete, bringing owned bounded ports to30;13/13 direct Style stubs remain implemented and337 owned rules still await their full gate. The shared layout helper retains the complete ArrayAlignment first/final replay and original Unicode checks. Additional source/version/encoding/directive/wire-format families and combined-cop scheduling remain outside this bounded claim. ArgumentAlignment is next and requires original-verified logical ownership of command-call arguments, not an assumption that raw tree-sitter siblings are Ruby arguments.


## ArgumentAlignment source/role prerequisites (in progress)

All50 unchanged pinned1.65.1 ArgumentAlignment examples now pass in the original Ruby environment, producing62 reference states. The capture runner restores the original spec/core_ext/string.rb strip_margin utility rather than modifying the upstream examples, and records sibling HashAlignment compatibility options and indentation width. These observations have not yet been declared native cop parity.

Imported the framework's command_unary_plus_call adapter from74b7bb0 and canonical raw keyword grouping/from_nodes from71223ed (based on Performance ba767c5). Existing original replays pass:318 lexer-role observations,234 valid/error-free command-bound observations and26 ordinary grouping observations through both grouping APIs. The six original-valid native-error command shapes remain explicitly unsupported by the command adapter. The raw grouping API does not normalize command ownership and is not treated as doing so. The core test walks nodes locally to preserve its existing traversal API's lifetimes; no production grouping algorithm changed.

Original AST probing confirms that outer(foo +1,other) has one outer argument and two inner foo arguments, that block passes change implicit-hash flattening, and that eligibility also covers attribute-setter Hash RHS and operator sends with a first multi-pair Hash argument. Static regexp =~ uses match_with_lvasgn even without named captures, with a Ruby3.3 static-string-part boundary; it must not be mistaken for an eligible send. Logical ownership and these role branches are the next implementation work. No ArgumentAlignment completion or additional bounded-port count is claimed.


## ArgumentAlignment implementation checkpoint (CLI gate pending)

The native port now matches all62 states from50 unchanged upstream examples. It implements ordinary/safe-navigation/index/operator sends and attribute-setter Hash RHS eligibility, command-tail ownership, both implicit-hash flattening policies and block-pass behavior, single braced-Hash eligibility, selector-line fixed indentation, Unicode display columns, typed sibling HashAlignment compatibility options and global-width fallback. The obsolete align_items implementation is removed after all three consumers moved to the original_alignment cop helper.

The original role/flattening property set contains127 observations:104 original-valid/error-free native cases match exactly,9 original-invalid probes are kept as evidence, and14 original-valid native-error states remain explicitly outside the adapter contract. These include two signed commands and12 versioned heredoc-in-regexp states. They are deferred parser/adapter gaps, not passing parity cases or literal safeguards. Static-regexp dispatch follows original Parser3.3.5.0 builders/default.rb#match_op/#static_regexp_node, including the Ruby3.3 boundary; the native-error heredoc forms are not claimed as covered.

The additional368 original correction states comprise324 exact supported states,16 supported literal-value safeguards and28 deferred native-error states. Together with the upstream62, all402 supported diagnostic/message/full-range states pass:386 exact first corrections plus16 intentional literal protections. Every original input/output was scanned through the original AST, and all16 changed-literal cases have reviewed native expected outputs whose literal values match the original input. Combined regexp/symbol cases and callback containment/order are included. Deferred states remain preserved separately for a non-allowlisted failing comparison; coincidental no-offense results are not counted as coverage.

The complete native checkpoint passes922 tests, one pre-existing ignored, followed by the additional independent correction replay. Actual config transport preserves sibling style arrays and disabled state, and width fallback has resolution assertions. Independent CLI, no-correction/width/compatibility controls, production scope, CRLF and fresh final-state gates remain pending. Existing redundant-disable auditing remains baseline-disabled for this cop; directive parity is unvalidated and remains queued. No new audit exception or generic scheduling change was introduced. No additional completed bounded-port count is claimed yet.

### ArgumentAlignment independent CLI controls checkpoint

The nine detection/autocorrection metadata controls, twelve own/global indentation-width precedence modes and twenty-four legitimate sibling HashAlignment compatibility modes all match the pinned original through the real CLI. The compatibility runner passes only public config: enabled/disabled siblings, colon/rocket separator styles, style arrays and both argument styles. It therefore checks resolver injection independently of the synthetic fixture flags. Eight actual Wistia-config engine/mode probes validate ordinary and nested unary-command corrections and clean stable final states.

A further32 original CRLF states match diagnostics/messages/full ranges and bytes. The unchanged50 original examples rerun successfully and regenerate all62 states identically. Eight additional later-argument unary-command probes are rejected by the original parser; their diagnostics are retained separately as invalid-source evidence, and no speculative recovery/ownership behavior was added. The broader CLI matrix and fresh final-state gate remain in progress; the cop is not counted complete.

### ArgumentAlignment fresh final-source review checkpoint

A separate native aggressive-correction replay is stable, and a fresh pinned-original replay now covers382 unique final native source/configuration states. Every final source parses in the original; no further original correction is emitted. Four document-comment states retain matching original no-op offenses. The native replay verifies their diagnostics/messages/full ranges as well as unchanged bytes, rather than equating an exit code with an offense comparison. The full native checkpoint passes925 tests, one pre-existing ignored, followed by all11 focused ArgumentAlignment tests including this final-state replay. The broader real-CLI original/native mode matrix remains running, so bounded completion is still pending.

### ArgumentAlignment final bounded gate

The434 supported first-pass states have exact diagnostics/messages/full ranges:418 exact corrected-byte states and16 reviewed literal-value safeguards. The pinned real CLI matrix completes868 safe/aggressive modes with836 byte matches, all868 exits and inspected-file scopes matching,1,736 stable second passes and zero cop errors. The32 byte differences are exactly the16 safeguards in both modes, and every native guarded output matches its reviewed safe bytes. Both engines finish860 modes clean and8 modes retaining offenses; this is not an all-clean result.

The separate fresh-original replay covers382 unique final native source/configuration states with matching diagnostics/messages/ranges and unchanged bytes, zero original parse errors and zero further edits. Four unique document-comment states retain original no-op offenses. The50 unchanged original examples regenerate all62 states identically. Nine metadata controls,12 width-fallback modes,24 legitimate sibling-config compatibility modes and8 actual Wistia-config engine/mode probes pass. Full native validation passes925 tests/one ignored, followed by all11 focused tests including the additional final-source replay. Recently changed code was separately reviewed against the complete original ArgumentAlignment and Alignment algorithms, original callback roles, version-dependent static-regexp dispatch, config/safety branches and correction ranges; no fixture code is executed.

The separate28-state native-parser-error corpus is measured without suppressing failures:56 modes produce28 corrected-byte differences across14 states, while all exits/scopes match and112 second passes are stable. The28 matching bytes in that corpus are coincidental and are not counted as covered native roles or cop parity. Generic syntax recovery remains explicitly deferred. The cop's pre-existing disabled redundant-directive audit remains unvalidated and queued. Additional source/version/encoding/wire-format families and combined-cop scheduling remain outside this bounded claim.

The separately reviewed bounded ArgumentAlignment port is complete, bringing owned bounded ports to31 with13/13 direct Style stubs implemented.336 owned rules still await their complete source/parity gate, alongside the documented limits of completed bounded ports. Next is Style/TrailingCommaInArguments: its existing implementation advertises autocorrection but emits none. All70 unchanged pinned original examples already pass and produce90 reference states; no native port for that rule is claimed yet.

## TrailingCommaInArguments implementation checkpoint (CLI gate pending)

Reviewed pinned1.65.1 Style/TrailingCommaInArguments, the complete TrailingComma mixin, ConfigurableEnforcedStyle branches and PunctuationCorrector. All70 unchanged original examples pass and capture90 states. The native port now emits actual insertion/removal corrections, handles single-line and inline-close forms, exact messages/full ranges, safe-navigation/selector-less calls, original heredoc traversal, implicit keyword hash grouping and both comma policies. Method definitions, super/yield, index syntax and block-pass insertion controls are distinguished from ordinary sends. The obsolete private helpers are removed; last-item highlight geometry is reused by the already ported literal cops.

An additional201 original-valid independent states gives291 passing diagnostic/message/full-range states:275 exact first corrections and16 intentional safety differences. Coverage includes45 CRLF states, all three styles, selector chains, blocks/forwarding, comments, Unicode, nested calls, braced/implicit hashes and splats, heredocs and logical unary-command ownership. Twenty-seven original-invalid source/config probes remain recorded separately; they are not native parity coverage.

A separate original parse of every independent corrected source identifies16 corrections that turn valid input into invalid Ruby: a trailing comma after nested bare-command tails (including unary plus/minus, keywords, splats or a block pass) or explicit argument forwarding. Actual original malformed bytes and parser diagnostics remain in docs/evidence/trailing-arguments-original-invalid-corrections.json. Native retains each original offense but declines those structural insertions, preserving valid authored source. No rule or per-source allowlist hides the difference.

CRLF normalization is an explicit opt-in extraction of the existing layout helper, owned by a substantive correction. The three completed alignment consumers retain identical behavior; no generic engine or scheduling policy changed. All15 trailing-comma family tests pass, including both previously ported literal cop original replays. The preceding full-suite checkpoint passes928 tests/one ignored, followed by the added safety replay. The real CLI, correction metadata, production scope, final-source and separate post-implementation review gates remain pending; this cop is not counted complete. Owned bounded ports remain31, with336 rules still awaiting their full gate plus documented existing limits.

### TrailingCommaInArguments final bounded gate

Post-implementation review covers the complete pinned original cop/mixin/corrector algorithms, all three configured styles, safety overrides, parenthesized call ownership, exact ranges, heredoc traversal and standalone/embedded argument forwarding. The unchanged 70 original examples regenerate all 90 states identically. The supported first-pass set has 303 exact diagnostic/message/full-range states: 283 byte-exact states and 20 explicit safeguards. Independent guard controls distinguish ordinary bare commands from parenthesized and block-bearing forms. All 20 declined original corrections are independently proven to produce original parser errors from valid input; their original bytes, diagnostics and safe native expectations remain recorded.

The real CLI matrix completes 606 safe/aggressive modes: 566 byte matches plus exactly 40 reviewed safeguard differences, all 606 exits/scopes matching, 1,212 stable second passes and zero cop errors. Every guarded native CLI result matches its reviewed safe bytes. Both engines finish 566 modes clean and 40 with offenses. Ruby's guarded outputs fail the pinned original parser; native retains valid source and the original cop offense instead. These outputs are intentionally different and are not advertised as correction parity.

Fresh original replay covers 258 unique final native source/configuration states. Every native final source parses in the original: 238 are clean with no further edit, while 20 retain the intentionally declined original offenses/corrections. The native replay verifies final diagnostics/messages/full ranges and safe bytes. Eighteen LF/CRLF metadata controls verify detection-only, disabled correction and Safe/SafeAutoCorrect overrides, including preservation of CRLF when correction is disabled. Eight actual Wistia-config engine/mode probes validate ordinary calls and heredocs with clean stable final states. Full native validation passes 930 tests/one pre-existing ignored, followed by the focused family and explicit native-tree coverage replay.

The final native-tree audit also detects three coincidental no-offense matches: one original-valid multiline anonymous-block-forwarding shape across three styles produces a native ERROR tree. Those three states and their six matching CLI modes are preserved separately and excluded from all supported totals above, as are their final sources. They require the deferred generic parser work; no private recovery or per-source exception masks them. All covered initial and final native trees are checked explicitly. Thirty original-invalid probes are separately retained and are not parity coverage. Directives, additional source/version/encoding/wire-format families and combined-cop scheduling remain outside this bounded claim.

The separately reviewed bounded TrailingCommaInArguments port is complete, bringing owned bounded ports to 32, with 13/13 direct Style stubs implemented. Another 335 owned rules still await their full source/parity gate, alongside the documented limits of completed bounded ports. The next review is Style/TrailingCommaInBlockArgs, including its original unsafe metadata and descendant-argument counting behavior.

## TrailingCommaInBlockArgs original-source preparation (port pending)

All 20 unchanged pinned original examples pass in Ruby and in the existing native handler. Reviewed the original block-only/lambda-literal eligibility, descendant counting of only arg/optarg/kwoptarg nodes, and the first two actual tPIPE tokens across the complete block expression (including a receiver block). Original role observations cover 95 valid sources and 107 AST blocks. Another 74 independent correction states are captured but not yet claimed as native parity. Thirty-two original-invalid source probes are separate. A valid empty-parameter receiver block followed by another block causes an original NoMethodError; its source and original error are recorded rather than reproduced or presented as passing parity.

The fresh original factory explicitly enables this cop, matching actual Wistia enforcement and the CLI runner; its standalone RuboCop default is disabled. Cop failures are captured through the original Team error report, with no swallowed exception or external telemetry call. The helper scripts pass actual Wistia lint checks. The native metadata trait encodes SafeAutoCorrect fallback, not the separate Safe flag, and must not be flipped merely because original Safe is false: public CLI defaults already match.

A 32-mode public metadata probe finds 29 byte/exit matches, one accepted nullable-safety mismatch (Safe:true plus SafeAutoCorrect:null incorrectly allows native safe correction), and two modes of an original-rejected config (explicit Safe:false plus SafeAutoCorrect:true). These are preserved as pending config differences, with ConfigLoaderResolver#merge, AutoCorrectLogic#safe_autocorrect? and ConfigValidator#reject_conflicting_safe_settings as the primary source branches. No production block-argument implementation or new completed-port count is claimed yet. Existing owned bounded counts remain 32 complete and 335 awaiting their full gate, plus documented limits.

## TrailingCommaInBlockArgs implementation checkpoint (CLI gate pending)

The source port replaces the identifier-count/text-suffix approximation with original parameter roles and actual pipe-token selection. All 107 original AST block observations from 95 native-parse-clean sources match, including exact descendant/top-level counts, lambda-literal roles, receiver-expression bounds, first pipes, comment handling and selected comma positions. Original callback ordering and duplicate-location behavior are reproduced locally to this cop. The concrete cop stays at its original registry position and preserves enforcement defaults; no generic engine scheduling policy changes.

All 94 supported diagnostic/message/full-range states pass: 20 unchanged original examples plus 74 independent states, with 83 byte-exact states and 11 deliberate single-parameter guards. The original cop can remove a required single-parameter comma because it counts descendants or selects a receiver block's parameters. A separate pure-parser proof enables the pinned parser's documented procarg0 representation flags and confirms all 11 removals change parameter binding; the original cop oracle keeps its unchanged flags. No fixture programs execute. Native keeps the original offense and declines those removals. Four existing legacy expectations now record the same general guard while preserving their actual original diagnostics and corrected bytes, with explicit reasons.

A nullable SafeAutoCorrect option is now resolved consistently with this cop's absent default: the accepted Safe:true/SafeAutoCorrect:null branch suppresses safe correction while allowing aggressive correction. All 30 original-valid public metadata modes match, and an Off/Safe/All real-engine control passes. Two modes of an explicit conflicting safety config are still rejected by original validation and accepted by native; those remain separately recorded pending a narrow config-validation follow-up. No safety trait flag was flipped.

The full native suite passes 935 tests, one pre-existing ignored; the binary builds. The semantic proof regenerates byte-identically after actual Wistia lint correction, and the Ruby helpers pass lint. The 32 invalid original probes and one original cop-crash source remain separate. Actual correction-mode CLI, production-scope, final-state and post-implementation review gates are pending; this rule is not counted complete. Owned bounded counts remain 32 complete and 335 awaiting their full gate, plus documented existing limits.

### Conflicting safety configuration validation follow-up

Native now rejects explicit Safe:false plus SafeAutoCorrect:true before linting, matching pinned ConfigValidator#reject_conflicting_safe_settings. Validation observes user and inherited sections before gem defaults/effective cop options are applied. Raw flags retain explicit nulls and section order; inherited conflicts, invalid parents, department/AllCops sections and first-error origin match the original. Required gem defaults are separately validated when loaded and do not masquerade as explicit user settings, so a default-unsafe cop with only SafeAutoCorrect:true remains valid as in Ruby.

The public CLI gate passes all 42 modes over 21 original-reference cases: exits, bytes and complete validation reasons match, including both conflicting-section orderings and plugin-default controls. All 26 original-rejected modes stop before modifying the source. All 32 standalone BlockArgs metadata modes now match, closing the previously accepted conflicting-config difference. The meaningful native replay loads actual config files/inheritance/plugin profiles and runs the real linter for accepted settings. Full native validation passes 936 tests/one pre-existing ignored. No fixture programs execute and no generic correction scheduling or syntax recovery was changed.

This validation batch is independent of the pending BlockArgs final-state/production gate. Existing owned bounded counts remain 32 complete and 335 awaiting their full gate, plus documented limits.

### TrailingCommaInBlockArgs final bounded gate

The unchanged 20 original examples regenerate identically. The supported gate has 94 exact diagnostic/message/full-range states: 83 byte-exact states and 11 explicit binding-preservation safeguards. All 107 original block-role observations from 95 native-parse-clean sources match, including the separately recorded original-error source's static roles. The separate semantic proof exposes the 11 changes to single-parameter procarg0 binding without executing any fixture program.

The real CLI completes 188 safe/aggressive modes with 177 byte/exit matches and exactly 11 deliberate aggressive-mode guards. All 188 inspected-file scopes match, all 376 second passes are stable and no supported case has a cop error. Default unsafe metadata leaves ordinary safe-mode offenses intact: original ends 144 modes clean/44 with offenses; native ends 133 clean/55 with offenses because it retains the 11 guarded offenses. Every native safeguard result matches its reviewed valid-source expectation.

Fresh original replay validates 83 unique final native source/configuration states. All final sources parse cleanly in both parsers: 72 are clean with no further original edit; 11 retain the intentionally declined original offenses/corrections. Five focused native tests pass after the full 936-test checkpoint/one pre-existing ignored. Eight actual Wistia-config engine/mode probes verify ordinary and guarded receiver paths, including their stable final offense state. All 32 metadata modes and 42 configuration/inheritance modes now match, including rejection before source mutation for conflicting settings.

The original empty-parameter receiver bug is measured separately in both modes: both linters preserve the source and inspect one file, but original reports a cop error and exits 1 while native avoids the error and exits 0. That intentional stability difference is not counted as passing cop parity. The 32 original-invalid probes, additional source/version/encoding/directive/wire-format families and combined-cop scheduling remain outside this bounded claim.

The separately reviewed bounded BlockArgs port is complete. Owned bounded ports now total 33, with 13/13 direct Style stubs implemented; another 334 owned rules still await their full source/parity gate, alongside documented limits. Next is Lint/TrailingCommaInAttributeDeclaration.

## TrailingCommaInAttributeDeclaration implementation checkpoint (CLI gate pending)

Both unchanged pinned original examples pass. The source port preserves the original registration position and matches 86 diagnostic/message/full-range states: 77 byte-exact states and 9 explicitly guarded corrections that the original parser rejects after rewriting valid input. It implements the original implicit-receiver accessor predicate and direct regular-definition last-argument role, canonical argument grouping, exact RangeHelp space/tab/newline offsets, CRLF normalization and warning severity. Explicit self/other/safe-navigation receivers, singleton definitions, grouped definitions and conditional/begin wrappers remain negative as in the original.

Six independent statement-separation controls distinguish escaped newlines from real statement boundaries, including backslashes inside comments, blank lines and CRLF. Native only removes a real separating comma when the result retains a statement boundary; parentheses, single-line definitions and unsafe continuations keep the original offense/source. The original malformed corrections and parser diagnostics remain recorded. No fixture programs execute.

All 84 independent valid native trees parse cleanly; four original-invalid sources and four one-definition-argument original cop errors are separate. The copied fresh reference helper explicitly enables the cop and records original Team errors. All 32 public safety/autocorrection metadata modes match, including null values for this cop's absent Safe/SafeAutoCorrect defaults. Full native validation passes 940 tests/one pre-existing ignored, the binary builds and the Ruby helper passes actual Wistia lint. The 172-mode CLI comparison is running; production scope, final-state and separate post-implementation review gates remain pending. No new completed bounded-port count is claimed: 33 complete, 334 still awaiting their full gate, plus documented limits.

### TrailingCommaInAttributeDeclaration final bounded gate

The original two examples regenerate identically, and all 86 supported diagnostic/message/full-range states pass. Seventy-seven states are byte-exact; nine retain the original offense/source because the original correction produces parser errors from valid input. Separate post-implementation review confirms accessor/receiver/definition roles, RangeHelp offsets, real statement boundaries across comments/continuations and nullable safety behavior against the original cop and mixins.

The real CLI matrix completes 172 safe/aggressive modes: 154 byte matches plus exactly 18 reviewed safeguard differences, all 172 exits/scopes matching, 344 stable second passes and zero supported cop errors. Both engines end 154 modes clean and 18 with offenses; original guarded outputs contain syntax offenses while native preserves valid input and the original cop offense. Those outputs are intentionally different.

Fresh original replay covers 79 unique final native source/configuration states, all parsed cleanly by both parsers. Seventy are clean with no further edit; nine retain the deliberately declined original corrections. Eight actual Wistia-config engine/mode probes verify reader/accessor and Unicode paths with clean stable final states. All 32 metadata modes pass. Four focused native tests pass after the full 940-test checkpoint/one pre-existing ignored; the binary and Ruby helper lint checks pass. No fixture programs execute.

Four original-valid single-definition accessor forms raise original cop errors. Their eight separately measured modes preserve the source and matching inspected-file scopes, but original exits 1 with a cop error and native exits 0 without one. This stability difference is not counted as passing parity. Four original-invalid inputs, additional source/version/encoding/directive/wire-format/LSP-context families and combined-cop scheduling remain outside the bounded CLI claim.

This separately reviewed bounded port is complete. Owned bounded ports now total 34, with 13/13 direct Style stubs implemented; another 333 owned rules still await their full source/parity gate, alongside documented limits.

## InterpolationCheck implementation in progress

Reviewed pinned RuboCop 1.65.1 `lib/rubocop/cop/lint/interpolation_check.rb` and all 11 unchanged original spec examples. The original explicitly documents that its delimiter rewrite can evaluate expressions that were literal data. The existing native port reproduced that behavior. This batch preserves the original offense while declining the rewrite, including the literal-template failure reported by the Performance agent (`Lint/InterpolationCheck`, not the string-literal style family). Raw original rewritten bytes remain in all reference fixtures; native expectations separately identify the safety difference. No fixture programs execute.

The original's regex uses immediate negative lookbehind (even multiple preceding backslashes suppress a marker) and does not cross LF. More importantly, physical multiline literals can be original Dstr nodes with unlocated Str parts, while a trailing newline alone still yields an eligible located Str. The new detector reuses the reviewed Ruby string-parts adapter rather than equating all native string nodes with original Str. It excludes symbol labels and all string descendants of heredocs, while preserving eligible strings nested in ordinary interpolation and concatenation. Full original range lengths are opt-in for this finding; other retained cops keep their existing range behavior.

The initial full native checkpoint passes 943 tests/one pre-existing ignored. Independent expansion covers 96 valid sources across Ruby 2.7/3.4 and records two original-invalid inputs separately. A percent-space delimiter range difference is under review, and public CLI, metadata, production scope, fresh final-state and separate final review gates remain pending. This rule is not counted complete: 34 completed bounded ports and 333 awaiting their full gate, alongside documented limits.

The percent-space case is now explicitly separated as native parse-clean AST span drift: two version probes remain in `docs/evidence/interpolation-check-native-span-drift.json`. Native's empty closing token/consumed newline does not represent the original literal boundary; no private recovery or successful parity claim is made for this syntax. The supported gate has 94 independent states and 13 captures from the 11 original examples. All 44 independent original rewrites increase the number of dynamic-string AST nodes; two also produce original parser errors. Pure parsing proves the activation without executing interpolation. An opt-in finding range override fixes original full-literal lengths for eligible literals containing a final physical newline. Other retained cops keep their existing highlight policy.

### InterpolationCheck final bounded gate

Separate post-implementation review confirms the original Str/located-token predicate, immediate-backslash negative lookbehind, same-line marker/closing-brace matching, ancestor heredoc exclusion, percent delimiters, concatenated literals, nested ordinary interpolation, Unicode and original full location lengths. All 11 unchanged upstream examples (13 captures) and 94 independent supported states match exact diagnostics/messages/ranges. Native deliberately reports guarded offenses as uncorrectable and emits no edits; the original rewritten bytes remain recorded. Full native validation passed 943 tests/one pre-existing ignored, followed by four focused final tests. The Ruby reference helper passes actual Wistia lint and regenerates byte-identically. No fixture programs execute.

The public CLI gate completes 214 modes: 166 byte matches plus 48 intentional aggressive-mode safeguards. All 214 scopes match and all 428 second passes are stable, with zero cop errors. Exit codes match in 168 modes; in 46 guarded modes original becomes clean while native retains the warning. The other two original guarded rewrites produce syntax offenses. Original ends 164 modes clean/50 with offenses; native ends 118 clean/96 with offenses. These are intentional differences, not claimed correction parity.

Fresh original replay covers all 107 final native source/configuration states. Every source parses cleanly in both parsers: 59 are clean with no further original edit and 48 retain the intentionally declined original offenses/corrections. All native findings in the replay are explicitly verified uncorrectable with no emitted edits. Thirty-two safety/autocorrection metadata modes contain 15 byte/exit matches (including two matching invalid-config rejections) and 17 guarded original rewrites; no unexpected differences remain. Eight actual Wistia-config probes cover expressions, quote selection and Unicode in both modes, with matching scope and stable final states.

The two original-valid percent-space delimiter span mismatches and two original-invalid inputs stay separate from supported coverage. Additional source/version/encoding/directive/wire-format families and generic syntax/cross-cop scheduling remain outside this bounded claim. The separately reviewed bounded port is complete: owned bounded ports now total 35, with 13/13 direct Style stubs implemented; another 332 owned rules still await their full original-source/parity gate, alongside documented limits.

## LiteralAsCondition source checkpoint (public gate pending)

Reviewed original RuboCop 1.65.1 `lib/rubocop/cop/lint/literal_as_condition.rb`, rubocop-ast 1.32.3 literal/basic-literal sets and method-identifier negation predicates. All 127 unchanged upstream examples pass in Ruby and yield 122 unique captures. Native matches those plus 496 independent original-valid states with exact messages/full ranges and explicit native parse-clean coverage. Three original-invalid probes remain separate; both tested parser versions reject the hypothetical subjectless case-match callback branch, so it is not claimed valid-source coverage.

The port covers condition callbacks for if/elsif/unless/ternary, modifier and post-test loops, boolean operand/parenthesis traversal, standalone prefix/explicit negations, case subjects and subjectless when groups, plus case-match binding exemptions (including nested cases). Regex/current-line matching and flip-flop conversion differ from ordinary case/explicit-send literal roles. Primitive arrays recurse through the original basic-literal distinction; dynamic strings, symbols, word arrays and heredocs reuse the existing string-parts adapter. Infinite-loop exceptions apply only to the original direct boolean condition. Global location deduplication matches overlapping original conditional/negation callbacks. Ranges and embedded source messages preserve original multiline/CRLF behavior.

Independent role probes also establish that current native grammar emits identifiers for all three magic names: original `__FILE__` is Str, `__LINE__` is Int and `__ENCODING__` is Const. The first two receive scoped literal classification; the encoding constant remains nonliteral. The cop emits no edits and all source bytes remain unchanged. No fixture program executes, including the parsed command-string specimens. Full-suite/build, public CLI, actual production scope, metadata and separate final review gates remain pending. Completed bounded counts remain 35, with 332 awaiting full review, plus documented limits.
