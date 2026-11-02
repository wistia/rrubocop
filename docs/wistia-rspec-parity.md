# Wistia RSpec parity implementation

This branch starts from the comprehensive audit at `a8c56ec`. It does not claim
complete RSpec parity. The exhaustive owned-rule ledger is
`wistia-rspec-parity-progress.json`; entries outside the tested batch remain open.

## First batch

Twelve former no-op rules now have implementations: MessageChain, Be, BeEmpty,
BeforeAfterAll, ContextMethod, DescribeSymbol, VerifiedDoubles, RemoveConst,
SubjectDeclaration, MissingExampleGroupArgument, MissingExpectationTargetMethod,
and UnspecifiedException.

The completion gate compares 136 cases against the original Ruby implementations
from rubocop-rspec 3.0.4, using Ruby RuboCop 1.65.1 and Wistia's effective options.
Seventy-five cases are extracted from the pinned upstream specs; the remaining
cases cover receivers, configured ignore branches, implicit method calls versus
local bindings, block forms, and negative controls. Original upstream spec files
are retained with their source version in `tests/fixtures/rspec-parity/upstream`.

Every case asserts detection, message, severity, range and correctability, exact
`-a` and `-A` output bytes, final offense state, and second-pass idempotence. The
review found and fixed chained-string and binary-operator location differences.
No source specimen is executed: syntax is compiled by the Ruby reference driver.

Run `cargo test --locked --lib pinned_rspec_original_ruby_detection_and_correction_parity`.
To refresh the reference results, use the actual Wistia Ruby bundle:

```sh
BUNDLE_GEMFILE=/path/to/wistia/Gemfile bundle exec ruby scripts/refresh_rspec_parity.rb /path/to/wistia/.rubocop.yml tests/fixtures/rspec-parity/cases.json tests/fixtures/rspec-parity/cases.json
```

The evidence is finite. It does not prove arbitrary custom RSpec language aliases
or all source encodings/combined-cop interactions. Remaining rules and missing
autocorrections must not be described as implemented until their ledger entries
are backed by corresponding original-Ruby differential checks.

## Second batch

Eleven more no-op rules now have implementations: ContainExactly, MatchArray,
HookArgument, EmptyMetadata, DuplicatedMetadata, ImplicitExpect, Eq, EmptyOutput,
VerifiedDoubleReference, ExpectInHook, and ExpectInLet. This brings this branch to
23 implemented former stubs; 57 original stubs and 16 previously implemented
RSpec rules still need their own original-source parity review.

The strict original-Ruby matrix now has 244 cases across those 23 rules,
including 146 extracted upstream cases plus configured style/ignore branches,
quoted symbols, local-variable negative controls, receiver restrictions, and
block forms. All cases compare detection and correction as above. Separate
checks verify actual spec-file scope and explicit `AutoCorrect: disabled` for all
11 newly supported correction capabilities.

Seven comment-bearing correction cases are deliberately not declared exact
parity: Ruby deletes their comments, and its EmptyMetadata/DuplicatedMetadata
rewrites produce invalid syntax. Both Rust correction modes preserve those
sources and keep reporting the original offense. The original Ruby outputs,
including syntax-invalid outcomes, are retained in `safety-guards.json` and are
covered by a separate source-preservation test. They remain visible safety
differences, not exceptions counted as parity successes.

The original Ruby ImplicitExpect implementation prefers `is_expected.to_not`
when replacing `should_not`, owing to its inverted alternatives table. This
implementation matches that intermediate result; RSpec/NotToNot may normalize it
in a combined-cop run. The current per-rule matrix does not claim combined-rule
semantic equivalence.

## Third batch

Nine additional no-op rules now have implementations: DescribeClass,
DescribeMethod, ExampleWithoutDescription, ExpectOutput,
IdenticalEqualityAssertion, MultipleDescribes,
UndescriptiveLiteralsDescription, VoidExpect, and NoExpectationExample.
The branch now implements 32 of the original 80 RSpec stubs. Forty-eight stubs
and 16 previously implemented rules still await their own review.

The strict matrix now contains 413 cases across all 32 implemented rules. Of
these, 263 are imported from pinned upstream Ruby specs using Prism's decoded
string/heredoc values, without executing specimen code. Thirteen upstream
host-generated templates cannot be imported without evaluating their test
context; they are explicitly listed in `import-manifest.json` and are not counted
as compared cases. `scripts/extract_rspec_upstream.rb` makes this import repeatable.
The remaining 150 cases exercise production/default and alternate configuration
branches, local binding and receiver controls, symbols versus strings in
metadata/hooks, interpolated description prefixes, and AST-equivalent literal
forms. All 48 RSpec/RSpecRails unit tests pass on this branch.

This source review also fixed a false-positive risk from treating an interpolated
`"#{method_name}"` as a literal method-description prefix, retained the distinct
symbol/string branches of the original hook and metadata matchers, and reused
the existing Ruby-compatible literal decoder for equality assertions and
constant-looking descriptions. The literal decoder is exposed through a small
shared wrapper so other cop departments can reuse the same implementation.

The configured AllowedPatterns evidence covers Wistia's simple prefix patterns
and an alternate explicit prefix. Arbitrary Ruby-only regexp features in custom
configuration, all numbered-block forms, and every possible AST spelling are not
claimed as proven. These remain finite differential results, not a proof of
complete equivalence for each rule or the whole RSpec department.

## Fourth batch

Eight additional former stubs are implemented: ReceiveCounts, ReceiveNever,
EmptyHook, SharedExamples, VariableDefinition, VariableName, ContextWording, and
SkipBlockInsideExample. Forty original RSpec stubs are now implemented; forty
stubs and sixteen pre-existing implementations still require review.

The strict matrix passes 571 Ruby-comparable cases across all forty rules. The
pinned upstream import contains 387 specimens, with thirteen host-generated
templates still unimported. Two upstream EmptyHook examples use `yield` outside a
method and fail RubyVM compilation; their explicit original errors are retained
in `cases-invalid-originals.json` and they are not counted as valid comparisons.
Nine safety differences are recorded separately. All 48 RSpec/RSpecRails tests
pass, including spec-file scope and disabled correction controls for sixteen
new correction capabilities.

This batch reuses the shared cached local-binding classifier, removing the
RSpec-only lexical scan. New original-engine probes cover `send` versus safe
navigation `csend`, configured name/description styles and ignore patterns,
metadata type distinctions, and alternative numeric spellings.

One original-Ruby bug is deliberately not copied: ReceiveCounts detects integer
`0x1` as one but uses decimal `source.to_i` while correcting it, changing
`.exactly(0x1).times` into `.twice`. Rust correctly uses `.once`. The trusted
`scripts/rspec_parity_count_semantics.rb` replay proves that original and Rust
accept exactly one call, while Ruby's correction accepts two instead. Observed
outcomes are retained in `receive-counts-runtime.json`; the diagnostic and both
engines' corrected bytes remain explicit in `safety-guards.json`.

The other eight safety cases retain comments rather than copying their deletion
or invalid syntax from the Ruby correction. The EmptyHook comment case is
explicitly classified here, rather than silently omitted from the upstream
matrix. These safety cases are differences, not exact-parity successes.

## Batch 5: separation and metadata

Six more original stubs are implemented: EmptyLineAfterExample,
EmptyLineAfterExampleGroup, EmptyLineAfterFinalLet, EmptyLineAfterSubject,
IsExpectedSpecify, and SortMetadata. The pre-existing EmptyLineAfterHook has also
been replaced and reviewed against the original implementation. Forty-six initial
stubs and one existing implementation are now covered; thirty-four initial stubs
and fifteen existing implementations remain open.

This batch's distinct post-implementation review used the pinned Ruby source and
all importable upstream examples, including the shared EmptyLineSeparation,
FinalEndLocation, and Metadata algorithms. Original Ruby checks exposed delayed
heredoc endpoints and the Metadata mixin's treatment of the final argument. Those
were corrected before counting parity. Explicit extra cases cover multiple
heredocs, one-liner configuration in both directions, safe navigation, numbered
blocks, static nested metadata, dynamic metadata values and keys. The existing
string decoder supplies heredoc terminator positions; whitespace fixes do not
insert bytes inside heredoc bodies.

SortMetadata reports dynamic expressions but refuses to reorder their evaluation.
Three explicit comparisons (one upstream and two additional key/value probes)
record that difference. The trusted metadata semantics replay proves that the
original source and retained Rust source evaluate z before a, while Ruby's
correction evaluates a before z. These are safeguards, not parity successes.
The other nine earlier safeguards remain explicit. The matrix has 685 strict
original-Ruby cases across 47 reviewed rules, plus 12 safety differences. The
upstream import contains 489 specimens; thirteen host-generated templates and
two invalid original yield specimens remain documented limitations. Disabled
correction controls now cover 23 added or repaired correction capabilities.

## Batch 6: shared context, implicit subject, and instance spies

SharedContext, ImplicitSubject, and InstanceSpy replace three more original stubs.
The reviewed total is now 49 original stubs plus the repaired EmptyLineAfterHook,
with 31 original stubs and 15 existing implementations still open. The strict
matrix contains 735 cases across 50 reviewed rules; the same 12 explicit safety
differences remain separate. Correction-disable controls cover 26 capabilities.

The separate post-implementation source review covered both shared-context
replacement directions, recursive setup/example searches, all four implicit
subject styles and `its` exceptions, and assignment/matcher linkage for instance
spies. The original upstream ImplicitSubject context configurations are restored
explicitly in imported cases; Ruby `let` blocks are never evaluated by the
fixture importer. Additional probes cover helper-method use outside examples,
local variables named subject, numbered blocks, safe navigation, unrelated spy
variables and comments between a double and `.as_null_object`. The latter are
retained byte-for-byte while replacing only the intended method spans.

`scripts/rspec_parity_subject_spy_semantics.rb` runs trusted RSpec scenarios. Both
original null-object doubles and corrected spies return themselves for an unstubbed
known method and record exactly one received call. Two real examples prove that
implicit and explicit subject forms each construct the subject once. Results are
stored in `subject-spy-runtime.json`. This finite proof is supplemental to the
original-engine diagnostic/range/correction/idempotence comparisons, not a claim
that every arbitrary Ruby program has been exhaustively explored.

## Batch 7: around hooks and implicit block expectations

AroundBlock and ImplicitBlockExpectation replace two more original stubs. The
reviewed total is 51 original stubs plus one existing rule, with 29 original stubs
and 15 existing implementations still open. There are 775 strict original-Ruby
cases across 52 reviewed rules; 12 safety differences remain separate.

The distinct source review covered optional/rest and numbered hook arguments,
permitted proxy call/run and argument/block-pass/yield usage, the symbol-only hook
scope, lambda/proc/Proc.new forms, nearest subject lookup and overrides, nested
example groups and safe-navigation branches. Additional cases exercise valid
method-local yield rather than executing any arbitrary fixture. A third upstream
specimen contains top-level yield and fails RubyVM compilation; its explicit
error remains in the invalid-originals fixture and it is not counted as parity.
The upstream import now retains 553 specimens, with thirteen host-generated
templates still explicitly unimported. Neither of these two rules autocorrects;
the existing correction-disable count remains 26.

## Batch 8: six existing implementations repaired

BeEq, BeEql, BeNil, ClassCheck, NotToNot, and SingleArgumentMessageChain now pass
the original-Ruby gate. Existing receiver/arity false positives, incorrect or
missing offense ranges, bare-method omissions, BeEql's positive-expectation
requirement, signed/quoted literal omissions and missing single-key-hash
correction were repaired. Both style directions and unsafe correction metadata
are exercised. The total is 51 original stubs plus seven existing implementations
reviewed, leaving 29 original stubs and nine existing implementations open.

The strict matrix has 836 cases across 58 reviewed rules, with 15 explicit safety
differences separate. Three additional safeguards retain comments in BeNil and
array-chain rewrites, and render quoted hash keys as valid inspected symbols
rather than copying Ruby's invalid `receive(:hello world)` output. All safeguard
cases now also check final offense state and idempotence. The standard disabled
correction controls cover 32 capabilities. The pinned import contains 601
specimens; thirteen generated templates and three invalid yield originals remain
explicit limitations.

Distinct source review covered exact send patterns, method/local-variable
boundaries, literal-node classification and decoded-dot checks, hash response
construction, receiver preservation and selector ranges. Additional formatter
probes cover Unicode, NUL, DEL, and the Ruby special symbols `!@` and `~@`.
The trusted chain replay proves that single-message hash/array chains and their
corrected receive/and_return forms produce identical repeated responses for nil,
integer and string values. This proves those ordinary RSpec scenarios, while
unsafe BeEq/BeEql corrections still require `-A` as in original Ruby.

## Batch 9: repeated descriptions

RepeatedDescription and RepeatedExampleGroupDescription replace two more initial
stubs. The reviewed total is 53 original stubs and seven existing implementations;
27 original stubs and nine existing implementations remain open. There are 879
strict comparisons across 60 reviewed rules and the same 15 explicit safeguards.
The pinned import retains 634 specimens; thirteen host-generated templates and
three invalid original yield specimens remain outside the strict matrix.

Separate original-source review included ExampleGroup's scope boundaries,
Example/Concept equality and SkipOrPending's direct-body matching. Additional
probes check implicit/explicit metadata hashes, hash labels versus symbol keys,
hexadecimal versus decimal literals, equivalent its block syntax, distinct method
names, nested ordering and direct versus multi-statement pending bodies. The
original numbered-block matcher groups missing description captures together;
that detection quirk is explicitly covered by two original-engine probes.

Native diagnostic arrays are compared in the same source-location order used by
the actual parallel CLI and the repository's existing assertion helper. Raw
lint_bytes traversal order is not CLI output order. Counts, every message/range,
correction capability, correction bytes, final offense state and idempotence
remain exact comparisons. The normal correction-disable count remains 32 because
neither of these two rules corrects source.

## Batch 10: repeated example bodies and includes

RepeatedExample, RepeatedExampleGroupBody, and RepeatedIncludeExample replace
three more initial stubs. The reviewed total is 56 original stubs plus seven
existing implementations, leaving 24 original stubs and nine existing
implementations open. The matrix contains 965 strict cases across 63 rules,
with 15 explicit safeguards and 32 correction-disable controls unchanged.

Separate original-source review included the RSpec Node extension's
recursive_literal_or_const? classification, group-scoped example lookup, body
and metadata signatures, const-only group discriminators and direct skip/pending
statements versus skip/pending blocks. The include-rule upstream templates now
expand all three known include_method values using closed, Prism-only
substitution; arbitrary host Ruby is never executed. This increases the pinned
import to 707 specimens while leaving the earlier thirteen unrelated templates
and three invalid original yield cases explicit.

Additional original-engine probes verify metadata spelling equivalence,
no-argument versus described examples, iterator scopes, constant groups, empty
groups, dynamic/blocked include exclusions and nested composite literals.
Rational spellings use exact BigInt numerator/denominator reduction: Ruby groups
1r with 1.0r, but keeps 1i and 1.0i distinct. Static regex equality preserves flags
while normalizing their order and delimiters, matching /a/im versus /a/mi and /a/
versus %r(a). These concrete branches pass the original engine; dynamic regex
interpolation remains conservatively compared and broader AST equality is still
a documented finite-evidence boundary for future shared-helper consolidation.

The follow-up regex review closes the specific dynamic-interpolation boundary
identified in batch 10. Original Ruby confirms repeated bodies using
/#{pattern}/ versus %r(#{pattern}) and /a\/b/ versus %r(a/b). The comparator now
compares interpolated expression nodes structurally and removes only escaped
literal delimiters; distinct /i and /m flags remain distinct. Three more strict
cases bring the matrix to 968 comparisons across the same 63 reviewed rules.

## Batch 11: predicate style, focus, and inclusion style

RedundantPredicateMatcher replaces another initial stub. Existing Focus and
ItBehavesLike implementations are repaired and reviewed. The reviewed total is
57 original stubs plus nine existing rules; 23 original stubs and seven existing
implementations remain open. The strict matrix has 1,006 comparisons across 66
rules, 15 explicit safeguards and 35 correction-disable controls. The pinned
upstream import retains 737 specimens, with thirteen earlier templates and three
invalid yield originals still explicit.

Separate original-source review covered block/numbered-block distinctions,
be_all's send-only first argument and report-only correction metadata, all focus
aliases and metadata forms, method-definition and chained-call exclusions,
explicit RSpec versus unrelated receivers, both inclusion styles and send versus
safe-navigation behavior. Original Ruby marks bare focus and be_all report-only;
those per-offense correction capabilities now match exactly.

The trusted inclusion/predicate replay runs four actual RSpec examples and proves
that both inclusion methods preserve shared-group setup/helper behavior and that
be_include/include both match the same collection. RSpec core defines the two
inclusion methods through the same nested-shared-group builder, with intentionally
different report labels; the style correction preserves that original behavior.
The runtime script passes original Ruby lint without changing the original forms.

## Batch 12: file suffixes and constant declarations

SpecFilePathSuffix replaces another initial stub; LeakyConstantDeclaration's
existing implementation is repaired and reviewed. The total is 58 original
stubs plus ten existing implementations reviewed, leaving 22 original stubs and
six existing rules open. The strict matrix contains 1,029 comparisons across 68
rules, with 15 explicit safeguards and 35 correction-disable controls unchanged.
The pinned import retains 751 specimens, thirteen earlier templates and three
invalid original yield specimens remain explicit limitations.

Original-source review covered global offense placement, top-level group and
namespace wrappers, shared-example exclusions, ordinary versus numbered-block
ancestry, class/module ranges and direct/scoped/multiple/operator constant
assignments. A dedicated original-engine probe verifies a constant assignment in
a group's call arguments, which Ruby still considers beneath the group's block.
The shared ancestry helper was reused rather than leaving an unused duplicate.

The Prism importer now reads upstream global-offense filenames literally. Their
suffixes and relative components are preserved beneath /tmp/spec so production
Include scoping is exercised; the importer does not evaluate filename-producing
Ruby. No source correction or runtime fixture execution is introduced by either
of these detection-only rules.

## Batch 13: file path format and source roles

SpecFilePathFormat replaces another initial stub. The reviewed total is 59
original stubs plus ten existing implementations; 21 original stubs and six
existing rules remain open. There are 1,081 strict comparisons across 69 rules,
15 explicit safeguards and 35 correction-disable controls. All 48 RSpec/RSpecRails
tests pass. The pinned import retains 789 specimens; thirteen earlier templates
and three invalid original yield specimens remain explicit.

Separate source review covered the Namespace/FileHelp/TopLevelGroup algorithms,
ASCII camel/acronym transforms and method sanitization, custom transforms,
IgnoreMethods and exact symbol-only IgnoreMetadata behavior. All 38 upstream
cases retain their filenames; nearest-block literal filename bindings are resolved
with Prism and the original File.expand_path stub result is materialized as an
explicit path. No Ruby filename expressions or mocks are executed. Added probes
cover Unicode method strings, absolute constants beneath namespaces, quoted hash
labels, metadata string negatives and mixed shared/ordinary top-level groups.
Cop-specific Include defaults now match the original file rules, including
malformed suffixes outside a spec directory.

The reference fixture snapshot now records the effective production cop options
used by the existing Ruby investigation, including its explicit correction-mode
override. Previously custom probes retained just their overrides, while Ruby
merged production defaults. Existing expected offenses and corrected bytes were
unchanged; backfilling those resolved options and rerunning the entire matrix
passed. This makes the configuration evidence explicit rather than relying on
coincidental matching defaults.

The framework agent's isolated static_symbol/bare_call_identifier helper is reused
(commit e49a15d, from 3ca6c2d), replacing the RSpec-only local-name whitelist.
Additional original-engine probes cover aliases, undef, method definition names,
block parameters, local variables named let, and defined expectation helpers.
Quoted labels are classified as symbols while string-arrow keys remain strings.

## Batch 14: subject scopes and overwritten setup

SubjectStub, RepeatedSubjectCall, and OverwritingSetup replace three initial
stubs. The reviewed total is 62 original stubs plus ten existing rules; 18
original stubs and six existing implementations remain open. The matrix has
1,158 strict comparisons across 72 rules, with the same 15 safeguards and 35
correction-disable controls. All 48 RSpec/RSpecRails tests pass. The pinned import
retains 849 specimens, with thirteen earlier host templates and three invalid
original yield specimens still explicit.

Separate original-source review covered top-level and nested groups, inherited
subject names, ordinary-let overrides, the original restricted Ruby AST traversal,
example-local memoization tracking and setup-name conversion. All thirteen
original SubjectStub top-level method variants are expanded through closed Prism
substitution. Additional probes cover conditionals versus assignments versus
methods, index arguments, eager-let behavior, string subject names, chained first
calls, aliases versus the implicit subject, conditional expectations and duplicate
calls within one expectation block.

Legacy Ruby AST classification matters: concatenated strings and multi-line
heredocs are dstr negatives for OverwritingSetup, while character literals and
single-line heredocs are str positives. The isolated helper change 27cc5ad reuses
the existing ruby_static_string decoder as a fallback; no second decoder is
introduced and its single-part guard is retained. Exact original diagnostic
messages, including a newline in a heredoc-defined name, are covered. These three
rules perform no source correction or fixture execution.

## Batch 15: pending reasons, mock responses, eager helpers, and spies

PendingWithoutReason and StubbedMock replace two more initial stubs. Existing
LetSetup and MessageSpies implementations are repaired and reviewed. The total
is 64 original stubs plus twelve existing rules reviewed, leaving sixteen
original stubs and four existing implementations open. The strict matrix has
1,242 comparisons across 76 rules; all 48 RSpec/RSpecRails tests pass. The same
15 safeguards and 35 correction-disable controls remain separate.

Distinct original-source review covered direct-parent begin/block mapping,
normal versus numbered examples, the final metadata argument and first offense
at a source range, response matcher arity, keyword hashes and block-pass forms,
group-scoped eager helper definitions, no-argument references, local-variable
shorthand and both spy styles. Additional probes distinguish a runner's response
block from a matcher argument's block, literal keyword hashes from positional
arguments, normal and splatted keywords, conditional/parenthesized statements,
multiple pending tags and source-faithful spy target messages.

One additional original upstream case nests numbered parameters in an outer
numbered-parameter block. RubyVM rejects that source; its precise syntax error
is retained with the previous three invalid original yield specimens, outside
the strict matrix. The pinned import now retains 916 specimens, with thirteen
remaining host-generated templates explicit. These four rules do not correct
source or execute fixture programs.

## Batch 16: named subjects and iterated expectations

Existing NamedSubject and IteratedExpectation implementations are repaired and
reviewed against their original algorithms and upstream specs. The reviewed total
is 64 original stubs plus fourteen existing rules, leaving sixteen original stubs
and two existing implementations. All 48 RSpec/RSpecRails tests pass with 1,292
strict original-Ruby comparisons across 78 rules. The separate fifteen safeguards,
35 correction-disable controls and four invalid original specimens remain explicit.

NamedSubject now uses the shared cached lexical binding model instead of a second
scanner. Review covers both styles, shared-example exclusions, normal versus
numbered blocks, first and inherited subject declarations, source-header reads,
keyword shorthand, assignment ordering and nested callback deduplication.
IteratedExpectation now matches the original each argument and expectation-shape
branches, including numbered parameters, qualified/safe calls and wrapped or
assigned expectation bodies. The original Ruby rule is report-only: its class has
no AutoCorrector. Unsupported native each-to-all rewrites and correction metadata
were removed; four existing correction scenarios remain report-only regressions.
Neither rule now rewrites source or executes fixture programs.

## Batch 17: overwritten subjects and redundant around hooks

MultipleSubjects and RedundantAround replace two initial stubs. The total is now
66 original stubs plus fourteen existing rules reviewed: eighty of 96 rules,
with fourteen original stubs and two existing implementations remaining. All 48
RSpec/RSpecRails tests pass with 1,318 strict original-Ruby comparisons, 37
correction-disable controls and 25 separately recorded safeguards.

Separate original-source review covers ExampleGroup traversal and its scope
boundaries, normal/numbered blocks, eager versus lazy declarations, subject!
correction metadata, named helper retention, callback proxies, block-pass hooks,
scope arguments, qualified and safe calls, nested bodies and run arguments.
The overwritten lazy subject is removed only when whole-line deletion preserves
neighboring statements and comments. Named declarations become lets; eager
subject declarations remain report-only, exactly as the original Ruby rule.

RedundantAround's original matcher does not check that the run receiver is the
hook proxy. It also removes qualified registration calls and dynamic scope
arguments. Native detection matches these original offenses, but corrections
retain the callback/registration when proxy-only behavior is not established.
These eight new hook safeguards and two whole-line subject safeguards remain
outside strict parity successes. The trusted subject-hook runtime script passes
four real RSpec examples, compares original/corrected memoized helpers and
proxy-hook execution, and demonstrates registration, worker and scope effects
that Ruby's broader deletion could discard. Fixture programs are not executed.

## Batch 18: yielding stub implementations

Yield replaces another initial stub, bringing the reviewed total to 67 original
stubs plus fourteen existing rules (81 of 96). Thirteen original stubs and two
existing implementations remain. The matrix contains 1,340 strict original-Ruby
comparisons, 38 correction-disable controls and 29 separate safety safeguards.
The native suite includes a regression for an additional original-rule crash.

Distinct source review covers stub discovery within send expressions, sole block
arguments, normal versus numbered parameters, safe and qualified calls, single,
multiple and parenthesized bodies, and exact argument joining. Original Ruby
moves yielded expressions from callback invocation to stub setup. Native reports
these offenses but retains dynamic arguments, constants and mutable allocations,
as well as comments. Trusted real RSpec mocks prove immutable yield sequences
and return values match, and demonstrate both changed evaluation timing and
mutable objects being unexpectedly shared by the broader Ruby rewrite.

The valid source `receive(:bar) { |&block| }` crashes the original rule with
NoMethodError at yield.rb:54 because it calls begin_type? on a nil body. Its source,
pinned version and error are retained in original-rule-errors.json, separate from
strict parity successes and invalid original syntax. Native preserves this source
without crashing in detection, safe correction or all-correction modes.

## Batch 19: static stub responses and shared expression comparison

ReturnFromStub replaces another initial stub. The reviewed total is 68 original
stubs plus fourteen existing rules (82 of 96), leaving twelve original stubs and
two existing implementations. All 49 RSpec/RSpecRails tests pass with 1,378 strict
original-Ruby cases, 39 correction-disable controls and 45 explicit safeguards.

Separate review covered both original styles, the RSpec recursive-literal helper,
static/dynamic values, chained and qualified calls, exact receive arity, numbered
and optional parameters, parenthesized bodies, comments, and single/multiline
heredocs including their report-only correction metadata. Native corrections
preserve immutable returns. Mutable responses, constant lookup and optional
parameter defaults retain their original timing and object sharing. Trusted real
RSpec mocks demonstrate all three hazards and verify immutable return parity.
Original invalid multi-statement and adjacent-block rewrites remain explicit
safeguards, outside strict comparison successes.

Shared helper import 78a3d03 reuses framework/core integer, send-range and typed
legacy AST comparison implementations. RSpec duplicate grouping now delegates to
strict eql_expression/eql_arguments; repeated-binding equality uses same_expression.
This removes the duplicate generic comparator and numeric/regex decoders. Eight
additional rule-level original probes cover regexp metacharacter escapes,
concatenated versus static strings, physical versus escaped newlines, and strict
versus ordinary rational/complex equality. The shared suite separately passes
48 original expression pairs (96 equality-mode assertions).

The comparator intentionally does not reproduce the upstream parser's stale
precomputed Node hashes after lexer mutation of continued heredoc strings. Exact
expression equality remains bounded to the documented tested AST families; this
known upstream hash artifact is separate from correction safeguards.

## Batch 20: actual expectation values and quoted symbol shapes

Existing ExpectActual is repaired and reviewed, bringing the total to 68 original
stubs plus fifteen existing rules (83 of 96). Twelve original stubs and one existing
implementation remain. All 49 RSpec/RSpecRails tests pass with 1,404 strict Ruby
comparisons, forty correction-disable controls and 61 separate safeguards.

Distinct original-source review covers exact actual/runner/matcher receiver and
arity patterns, routing exclusions, simple and complex literal classifications,
implicit keyword hashes, typed strings/symbols, both be forms and correction
metadata. Detection now covers complex/rational values, regular expressions,
hashes and original-source ranges while avoiding qualified and dynamic forms.
Scalar-parenthesized corrections match original bytes and final offense state.

The original dynamic-object swap reverses the receiver of ==, eql? or equal?.
Trusted RSpec matcher probes demonstrate eq/eql/equal/be changing from false to
true for asymmetric objects after the Ruby correction. Native retains those
comparisons and reports the original offense rather than silently changing a
test result. Sixteen new explicit safeguards, including upstream correction
specimens, remain outside strict successes. The scalar probe preserves its result.

Isolated helper commit 95fc00e extends the existing typed decoder to quoted symbols
and labels. Physical multiline symbols with multiple str parts remain dsym;
trailing-newline one-part symbols and escaped continuations retain sym semantics.
Six additional original pairs cover these branches and quoted-label versus hash
rocket equality: the shared suite now has 54 pairs and 108 mode assertions.

## Batch 21: declaration ordering and atomic movement

LeadingSubject is repaired and HooksBeforeExamples, LetBeforeExamples and
ScatteredLet replace three original stubs. The reviewed total is 71 original
stubs plus all sixteen existing implementations (87 of 96); nine original stubs
remain. All 49 RSpec/RSpecRails tests pass with 1,465 strict original-Ruby cases,
44 correction-disable controls and 81 explicit safeguards.

Distinct original-source review includes language predicates, nearest ordinary
block/root-group scope, marker aliases, qualified/numbered forms, include/shared
boundaries, block-pass arity, and declaration-name lookup. Shared movement follows
MoveNode/CommentsHelp/FinalEndLocation, preserving exact indentation, blank lines,
attached comments and delayed heredoc bodies. Directive attachments remain intact.
Multiple moves use explicit offense ownership to keep each insertion/deletion
atomic: the regression probes exposed native deletion loss when colliding
insertions were inferred as different owners. No scheduling engine was changed.

Twenty-one trusted real RSpec examples verify ordinary lazy/helper/hook movement
and demonstrate eager-hook ordering, named/implicit subject override, included
subject override and multiple-hook reversal hazards. Native preserves original
hook registration order; Ruby's simultaneous before-hook moves reverse it.
Other unsafe source-order changes and directive scope changes remain separately
recorded safeguards, outside strict parity successes.

Two further original defects are retained in original-rule-errors.json. A subject
inside a numbered root group crashes LeadingSubject#parent while looking for an
ordinary block ancestor. A same-line move raises Parser::ClobberingError. Native
preserves these sources, reports the applicable same-line offense, and passes
all three original-rule-error regression cases in each mode. The four earlier
invalid-original-syntax specimens remain separate.

Combined-cop scheduling is explicitly deferred by the user. Original
LetBeforeExamples declares autocorrect_incompatible_with [RSpec::ScatteredLet]
at lib/rubocop/cop/rspec/let_before_examples.rb:65; the native generic scheduler has
no equivalent metadata API. This batch makes individual-rule claims only and does
not claim combined-cop correction parity. The canonical integration follow-up is
maintained by the parent agent separately.

## Batch 22: description wording, spacing and decoder reuse

ExampleWording and ExcessiveDocstringSpacing replace two original stubs. The
reviewed total is 73 original stubs plus all sixteen existing implementations
(89 of 96); seven original stubs remain. All fifty RSpec/RSpecRails tests pass
with 1,614 strict original cases, 46 correction-disable controls and 88 separate
safeguards. The original Wording helper's 34 unit specimens also pass directly.

Distinct original-source review covers prefix/negation/case handling, pluralization,
custom transforms, ignored words, insufficient descriptions, Unicode blank and
space classes, direct/qualified/numbered/no-block scope, concatenation,
interpolation and heredocs. All 97 original cop specimens and the 34 helper-derived
rule controls are retained with their actual options; the four Workz contexts
restore their original DisallowedExamples setting explicitly.

Dynamic description text reuses the existing literal decoder through placeholder
parses, without executing interpolation or arbitrary fixtures. Corrections require
literal round-trip validity, equivalent interpolation ASTs and stable relative
__LINE__ positions. Seven additional safeguards retain source when Ruby changes
an interpolated string value, moves __LINE__, or creates invalid decoded-quote or
percent-literal syntax. Trusted runtime probes show preserved native interpolation
effects and the broader Ruby rewrite's changed values/line payloads; harmless
expression whitespace retains its value.

Shared checkpoints c9b46f9 and b10c9f3 import version-aware regexp decoding and
canonical dump/inspect/symbol formatting. The shared suite passes five tests,
including 56 original AST pairs and 68 regexp target-version observations.
RSpec formatters now delegate instead of duplicating quoting logic. Exact control
name probes also exposed Symbol#to_s encoding: ASCII-only symbols produce US-ASCII
inspection (x00/x01/x7F), while ordinary UTF-8 strings use u0000/u0001/u007F.
VariableDefinition now selects the original symbol-string inspection semantics;
the previous control-name regression remains unchanged and passing.

## Batch 23: metadata style and exact metadata scope

MetadataStyle replaces another original stub. The reviewed total is 74 original
stubs plus all sixteen existing implementations (90 of 96); six original stubs
remain. All fifty RSpec/RSpecRails tests pass with 1,663 strict original cases,
47 correction-disable controls and 92 separate safeguards. A non-test
cargo check --locked --lib also passes.

Separate original-source review covers both styles, the Metadata mixin and
RangeHelp whitespace/comma operations, including 24 upstream specimens. Controls
cover implicit/braced/empty hashes, multiple and ambiguous tags, second description
strings, quoted labels/symbols, splats, comments and exact metadata ranges. A shared
logical argument view models implicit keyword hashes without changing the parser.
Corrections use explicit offense ownership and validate the resulting syntax.

Post-implementation trusted RSpec 3.13.0 probes and Metadata.build_hash_from source
confirm that symbol tags override the final hash. Hash-style conversion preserves
this precedence, including existing false values. Symbol-style conversion can
incorrectly change a later false value or spread into true; those two concrete
value hazards, comment deletion and quoted tag identity form four new safeguards.
The runtime artifact records actual original and rewritten flag values.

Metadata eligibility now matches the original ordinary-block and configure
parameter/argument/lvar patterns. New negative controls also exercise existing
EmptyMetadata, DuplicatedMetadata and SortMetadata with numbered groups and
unsupported configure signatures, rather than silently broadening their scope.

The pinned original MetadataStyle raises TypeError for hash-style conversion of
`describe "x", :a, **metadata do ... end`: its empty-pairs path expects a brace
location that an implicit splat hash does not have. This fourth original-rule error
is retained separately. Native inserts the true pair after the splat, preserves
RSpec precedence, and verifies final offense state and idempotence in both modes.
The earlier four invalid-original-syntax specimens remain separate.

## Batch 24: expectation change forms

ExpectChange replaces another original stub: 75 initial stubs plus all sixteen
existing implementations are reviewed (91 of 96), with five initial stubs left.
The strict corpus now contains 1,685 original cases, alongside 109 explicit
safeguards and 48 correction-disable controls. All twenty original upstream
contexts were inspected with the original source and both configured styles.
Unsafe autocorrection metadata is retained; safe mode preserves the source.

Detection covers constant/bare method receiver distinctions, regular versus
numbered blocks, block parameters, getter arity and static symbol/string messages.
Corrections additionally validate the generated getter syntax and receiver AST,
preserve comments, and guard receiver capture, allocation identity and constant
replacement. A separate pinned RSpec expectations/mocks runtime artifact proves
stable-receiver controls and demonstrates changed call counts, local values,
mutable object identity and stub_const behavior in the broader Ruby rewrites.
Signed numeric receiver precedence is explicitly guarded. These deliberate
safeguards are excluded from strict correction equality claims. Combined-cop
scheduling remains deferred as requested.

## Batch 25: zero-delta change matchers

ChangeByZero replaces another initial stub: 76 initial stubs and all sixteen
existing rules are reviewed (92 of 96). The corpus contains 1,711 strict cases,
120 separate safeguards and 49 correction-disable controls. Original-source
review covers the three change aliases, argument and block patterns, integer-zero
spellings, compound and/or/&/| forms, exact ranges, custom NegatedMatcher and
conditional correctability. All sixteen upstream specimens are retained; four
custom matcher contexts explicitly apply the original host configuration.

Pinned RSpec expectations 3.13.2 source distinguishes actual_delta matching from
ChangeDetails#changed?, which also compares hashes. A trusted runtime probe shows
that Integer 1 becoming Float 1.0 passes by(0) and fails not_to change. The port
therefore preserves unknown getter corrections, while pure literal numeric getter
controls support both simple and custom compound correction. Comments, invalid
negated matcher identifiers and nonpositive runners are preserved. These explicit
safeguards are excluded from strict correction parity claims.

The original raises NoMethodError at change_by_zero.rb:121 for a standalone
change(Foo, :bar).by(0), because the matcher has no parent. This fifth original-rule
error is retained separately; native reports the offense without crashing or
changing the standalone matcher. Generic correction scheduling remains deferred.
