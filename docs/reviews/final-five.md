# Final five ports: independent review

**Follow-up:** All findings below are addressed in the [parity fixes and optimization record](port-fixes-and-optimizations.md). The remainder preserves the original review evidence.

Reviewed implementation commit `832a13f9a10842551c09eef37fe3dc513b81fbd5`. This is a review record; the findings below remain unfixed. New targeted comparisons supplement the previously passing 375 final-five reference cases and CLI correction checks.

| Priority | Cop | Confirmed discrepancy |
|---|---|---|
| P2 | Style/EvalWithLocation | Backtick command heredocs receive an incorrect safe autocorrection adding a physical source location for shell-generated code. |
| P3 | Style/EvalWithLocation | Character literals and the magic `__FILE__` source argument are missed. |
| P2 | Lint/OutOfRangeRegexpRef | Deferred heredoc interpolation is visited in the wrong evaluation order, causing missed and extra offenses. |
| P2 | Lint/OutOfRangeRegexpRef | Element assignment is treated as a `[]` match, incorrectly changing capture state. |
| P2 | Lint/ShadowedArgument | Nested/rest destructured parameters are not fully bound, so overwrites are missed. |
| P2 | Lint/ShadowedArgument | Pattern rebinding is flagged although pinned RuboCop does not flag it. |

Eval also conservatively refuses correction of pure parenthesized filename/line arguments. This leaves an offense outstanding but does not apply an unsafe edit. Existing intentional refusals for uncertain/effectful arguments remain appropriate.

The other four ports are diagnostic-only. No additional mismatch was found for MixedRegexpCaptureTypes in 59 valid regexp comparisons, or for NoUndiscoverableAutoLoadedConstants in 11 targeted comparisons. These probes do not establish exhaustive parity.

Detailed reproducers, source locations, coverage limits, and recommendations:

- [Eval and autocorrection](final-five/eval.md)
- [Regexp capture state](final-five/regexp.md)
- [Autoload and argument scope](final-five/autoload-shadow.md)
- [Performance of all 98 additions](ported-rule-performance.md)

Temporary raw probe paths in the individual reviews identify local execution evidence; the minimal reproducers are included in these committed reports. No implementation changes or broad test reruns were made during this review.
