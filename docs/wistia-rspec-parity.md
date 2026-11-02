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
