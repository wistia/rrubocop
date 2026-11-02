# Follow-up autocorrect hazard fixes

The independent audit identified three additional correctness gaps after the expanded discrepancy run was clean. This follow-up fixes their shared matchers and avoids repeated semantic-key serialization in SafeNavigationConsistency.

## Correctness

- The shared integer decoder accepts integer literals and a single unary operation on an integer literal. Nested unary expressions such as `--1` and `+-6` are excluded, matching the pinned Ruby AST shape instead of flattening signs. SlicingWithRange, RandomWithOffset and UnpackFirst no longer rewrite these expressions into different values.
- ClassEqualityComparison requires exactly one comparison argument and no arguments on the matched `.class` and class-name sends. This preserves side effects and valid overridden-method behavior. A block attached to the outer equality call is retained by limiting the edit to the send expression.
- HashExcept requires exactly one predicate-call argument and excludes attached predicate blocks. Rewrites no longer discard additional arguments or callbacks, or change filtering with custom key methods.

The scoped fixture contains 38 pinned references, including positive controls, repeated unary expressions, exact arity, attached blocks, comment preservation, and receiver-key ordering. Existing HashExcept comment-deletion safeguards remain precise per-example exceptions. Eight additional trusted runtime scenarios cover the reproduced slicing/random/indexing and valid overridden-method cases. Generated examples remain excluded from runtime execution.

## Efficiency

SafeNavigationConsistency indexes the first safe call for each semantic receiver key, then performs one lookup per unsafe condition. It preserves the previous first-match ordering and avoids receiver-key allocation for logical chains with no unsafe conditions. This removes repeated key serialization inside a nested scan.

Cache rules revision is 41.

## Verification

- 822 Rust tests pass, with one normal-suite audit probe ignored. The central corpus contains 2,998 pinned examples, plus 30 separate pinned references.
- Expanded live audit: 4,719 selected examples, 4,680 compilable comparisons across 98 cops, zero new failures. Invalid mutations and originals retain separate classifications.
- 23 trusted runtime scenarios across 15 cops verify native behavior, including all reproduced hazards.
- 114 release CLI mode checks pass for the 38 new regression rows.
- Formatting and ABC checks pass, with no new Clippy diagnostic signatures; the existing 223 test-target diagnostics remain.

An interleaved local benchmark of three uncached runs per binary preserved exact diagnostics. The 300-safe/300-unsafe synthetic chain improved from a median 110 ms to 72 ms; smaller and all-safe cases were approximately unchanged amid process-startup variation. This is a synthetic result, not a full-codebase timing.
