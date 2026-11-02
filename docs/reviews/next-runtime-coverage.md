# Next runtime coverage batch

This batch adds 24 explicit, self-contained execution scenarios for six rules that previously lacked scenarios in the unified runtime harness: IdentityComparison, SafeNavigationConsistency, RedundantConditional, RedundantSort, RaiseException and DuplicateRequire.

- IdentityComparison: identical/distinct objects, receiver evaluation order, overridden object_id/equal? behavior, and attached outer equality blocks.
- SafeNavigationConsistency: nil short-circuiting, non-nil behavior, repeated receiver evaluations, and independent receivers in a logical chain.
- RedundantConditional: operand evaluation order, inverse comparisons, elsif rewriting and retained comments.
- RedundantSort: min/max extraction, empty inputs, sort_by callback order and symbol-to-proc blocks.
- RaiseException: raise/fail forms and exception-message argument evaluation, with rescue covering both intended exception classes.
- DuplicateRequire: require-once behavior, different spellings referring to the same file, and repeated calls inside a block. The file is supplied explicitly in the sandbox fixture.

The new programs found a correction-boundary discrepancy: IdentityComparison removed an outer comparison block that pinned Ruby retains. The correction and diagnostic now stop at the send expression, preserving the block. Cache rules revision is 43.

The overridden object_id program intentionally records a runtime change shared with Ruby; it remains classified as an upstream hazard rather than a native-only regression. No broad exception was added to the harness. Generated programs remain unauthorized for runtime execution.

The six-seed differential pass additionally exposed two DuplicateRequire shapes where deleting a physical line removes a closing delimiter for an expression that started earlier. These now remain report-only instead of failing native autocorrection with invalid Ruby. The guard checks actual ancestor closing tokens and preserves ordinary standalone require removal. Both sources are pinned alongside the runtime scenarios.

## Verification

The six-rule differential audit with six mutation seeds per rule compares 618 compilable examples and 24 trusted runtime programs, with zero new failures. The normal suite passes 823 Rust tests (one batch probe ignored), including 3,304 central references plus 30 separate pinned references. All 78 release CLI checks for the 26 scoped rows pass. Formatting and ABC checks pass; no Clippy diagnostic signatures were added to the existing backlog.

The unified runtime manifest now contains 72 scenarios across 31 of the 98 added rules. Earlier scenarios for unchanged rules were verified in previous batches; this focused run executes the 24 scenarios for these six rules. Runtime changes matching the pinned reference remain separately classified upstream hazards.
