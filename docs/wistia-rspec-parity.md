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
