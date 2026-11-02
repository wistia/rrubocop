# Runtime coverage: remaining source-rewriting rules

This batch adds 28 trusted runtime programs for InterpolationCheck, MultipleComparison, RescueType, TopLevelReturnWithArgument, TrailingCommaInAttributeDeclaration and UriRegexp, plus three static comparison regressions.

## Confirmed fixes

UriRegexp now preserves a block attached to the original send, including its side effects, and excludes that block from the diagnostic message. A custom parser spy reproduced the old native-only callback loss against pinned Ruby 1.65.1.

MultipleComparison now excludes explicit bitwise `&`, `|` and `^` calls as well as infix forms, matching Ruby. Valid custom-operator programs demonstrate that the old rewrite duplicated middle-operand evaluation; all three operator controls now preserve the original call sequence. Existing safeguards against runaway explicit-comparison correction and invalid parenthesized attribute correction remain pinned.

## Runtime execution and coverage

The harness supports explicit `runtime.execution: file` for top-level return scenarios: it writes and loads `probe.rb` inside the fresh sandbox. Eval remains the default. Unknown execution modes fail the runner before the program executes, preventing false coverage. Three new integration tests cover file termination/ensure, argument side effects and watched files, and invalid execution modes.

Some rules intentionally repair behavior, such as interpolation, invalid numeric comparison chains and invalid rescue clauses. Runtime changes shared with Ruby remain separately classified rather than treated as native-only failures. Generated programs never gain execution authorization.

Verification: 823 Rust tests, 41 Python harness tests, and 93 release CLI mode checks pass. The six-rule audit compares 481 compilable examples and 28 runtime programs with zero new failures. All originals execute successfully. Rust and Ruby formatting/ABC checks pass; the existing Clippy backlog is unchanged.

The central corpus contains 3,396 references, plus 30 separate pinned references. Unified runtime coverage contains 155 scenarios across 49 added rules. All 48 added cops that rewrite Ruby source now have at least one scenario; the remaining declared autocorrectable cop, Migration/DepartmentName, changes comments and is better checked through directive behavior tests. Finite runtime coverage does not establish arbitrary-program equivalence.

Cache rules revision is 45.
