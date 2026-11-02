# Expanded audit discrepancy fixes

This follows up on the 153 discrepancy records across 64 generated examples recorded in [the independent audit](independent-parity-fixes.md).

## Matching and correction fixes

Block-sensitive matchers now distinguish attached blocks from ordinary calls. IdentityComparison, ClassEqualityComparison, FloatComparison, SafeNavigationWithEmpty, and RandomWithOffset follow the pinned Ruby exclusions. RedundantWithIndex/Object keep a diagnostic without editing when removal would create adjacent blocks. Chained block-parameter token selection follows the reference cop.

Numeric indices use decoded integer values, including hexadecimal, binary, octal, explicit decimal and signed forms. SlicingWithRange and UnpackFirst recognize the same numeric shapes. Explicit operator calls are recognized by MultipleComparison, RandomWithOffset and CompoundHash.

Empty calls with comments compare by semantic structure while preserving the distinction between local variables and bare method calls. SafeNavigationConsistency and DuplicateElsifCondition use this identity. Comment guards permit corrections when comments are retained, while HashExcept, OrAssignment and RedundantConditional preserve comments that a rewrite would discard. Attribute declarations use semantic arguments and guard the parenthesized syntax hazard.

Migration option detection decodes static symbol keys, and insertion accepts those keys safely. Existing decoded charset/collation options are recognized instead of adding conflicting defaults. Splat, dynamic and string-keyed option bundles retain manual correction safeguards.

## Intentional differences from Ruby

The suite pins each safeguard to an exact source, diagnostic expectation and corrected source. No broad audit exemptions were added.

- Multiline migration corrections insert inside parentheses; the Ruby custom cops append outside them and generate invalid Ruby.
- RedundantWithIndex/Object remain report-only when the receiver has its own block; removing the selector would leave adjacent blocks.
- Parenthesized attribute declarations remain report-only when removing the comma produces invalid Ruby.
- Explicit inner comparison calls remain report-only where the Ruby correction repeatedly expands its own argument.
- Useless explicit `.[](...)` stays report-only where Ruby removes just `[]`, converting indexing into `.call(...)`.
- Corrections that delete comments remain report-only; corrections that preserve them proceed.

The cache rules revision is 40. Runtime authorization remains limited to explicit trusted scenarios; generated inputs are never executed automatically.

## Verification

All 64 previously failing case identities are retained in the expanded run. 48 now have exact parity across the compared dimensions; 16 retain explicitly tested safeguards for unsafe upstream behavior or comment deletion. None of the 153 former failure records remains a failure.

- 822 Rust tests passed; one audit probe remains ignored during the normal suite.
- 2,960 central pinned references pass, plus 30 separate pinned references. The four scoped fixture sets contain 111 rows including repeated controls.
- Expanded live audit: 4,673 selected examples, 4,634 compilable comparisons, 98/98 cops, zero new failures. The 34 invalid mutations and five invalid originals remain separate classifications, never passing examples.
- 15 trusted runtime scenarios across 12 cops passed for native corrections. The new indexing scenario confirms Ruby's unsafe correction raises NoMethodError while native leaves the original behavior intact.
- 333 release CLI mode checks pass on the four scoped fixture sets.
- ABC checks report zero function or module complexity offenses. Clippy has exactly the same existing diagnostic signatures as the baseline (223 test-target diagnostics).

The final SlicingWithRange dispatch filter was added after the full batch native capture; all 63 scoped mode outputs remain byte-for-byte identical and the retained-rule pipeline test passes.
