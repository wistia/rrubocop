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
