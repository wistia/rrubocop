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
