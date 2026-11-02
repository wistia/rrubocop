# Wistia core cop parity work

Reference implementation: Ruby RuboCop 1.65.1, as pinned by Wistia. The starting
inventory and reproductions are the comprehensive audit of fork a8c56ec. No
retirement of enforced rules or relaxed parity exceptions is authorized here.

## Current scope

Twenty owned rules now have implemented behavior backed by original-Ruby
comparisons, including ten of the original thirteen direct-file Style
stubs, the additional delegated RedundantParentheses stub, and both absent
version-gated cops. HashSyntax shorthand branches and additional
UselessAssignment capture combinations remain under review. The exhaustive
367-rule inventory retains 347 queued validation entries; some already had
implementations. These bounded original-spec results do not establish
universal parity for arbitrary Ruby source.

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
