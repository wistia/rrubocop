# Runtime coverage: evaluation, regexps and definitions

This batch adds 29 explicit trusted scenarios for six previously uncovered rules: EvalWithLocation, RedundantRegexpCharacterClass, RegexpAsCondition, TrailingBodyOnMethodDefinition, TrailingMethodEndStatement and OrAssignmentToConstant.

The scenarios exercise eval bindings, class/instance state, heredocs and ignored blocks; regexp matching, Unicode, escaping, captures and substitutions; conditional matching against `$_`; method return values, singleton definitions, ensure and comments; and constant initializer evaluation. All cases explicitly enable autocorrection. Generated source remains unauthorized for runtime execution.

## Findings and fixes

The eval missing-location diagnostic included whitespace before an attached block. It now uses the send expression boundary, matching Ruby's highlight without changing correction behavior.

High-precedence `!`/`!!` regexp conditions remain report-only. The old correction changed operator binding and could raise NoMethodError; Ruby's repeated correction produces invalid syntax. The guard preserves the original behavior rather than reproducing either failure. Controls distinguish low-precedence `not`, which remains correctable, from explicit `~` matching, which should not be diagnosed as an implicit regexp condition.

The pre-existing method-body first-line-comment safeguard is retained with a precise pinned expectation. Constant reassignment changes shared with the pinned Ruby cop remain classified as inherited behavior; no native-only failure is hidden.

Cache rules revision is 44.

## Verification

- 823 Rust tests pass, with one normal-suite batch probe ignored; 3,333 central references plus 30 separate pinned references.
- Six-rule audit with six mutation seeds per rule: 519 selected examples, 512 compilable comparisons and 26 runtime programs, zero new failures.
- Final unary-operator follow-up: 73 compilable regexp-condition examples and nine runtime programs, zero new failures. It includes all affected regexp case identities from the six-rule run plus the new controls.
- All 87 release CLI mode checks pass for the 29 scoped rows. Formatting and ABC checks pass, with no new Clippy diagnostic signatures.

Unified runtime coverage now contains 101 explicit scenarios across 37 of the 98 added rules. This batch checks the 29 scenarios for the six selected rules; earlier unchanged scenarios were verified in previous batches. A finite set of runtime programs is not a proof of arbitrary Ruby equivalence.
