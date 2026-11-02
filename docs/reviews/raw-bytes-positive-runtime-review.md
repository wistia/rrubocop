# Raw-byte safety and positive runtime coverage review

This follow-up addresses the independent review of 2e634fb. The earlier scenario-presence totals did not prove that each cop successfully rewrote a runtime program. Reconciliation of historical artifacts found 40 cops with completed positive rewrites versus 49 with scenarios. Eight source-rewriting cops lacked a completed positive: the seven unchanged-only cops plus SafeNavigationWithEmpty, whose attempted rewrite raised an exception. DuplicateCaseCondition is diagnostic-only and is not expected to rewrite.

## Production fixes

URI regexp correction now uses surgical byte-span edits. It preserves the first argument's raw bytes and any attached outer block, while keeping the reference cop's first-argument behavior. Valid ASCII-8BIT files with raw 0xFF no longer panic in off, safe or all modes. Diagnostics use lossy UTF-8 display, while source corrections preserve original bytes; Ruby's JSON formatter cannot itself emit those invalid-UTF-8 diagnostic strings, so real CLI comparison uses quiet output and exact byte checks.

The analogous class-comparison, hash-conversion and min/max/unpack accessor boundaries now measure raw bytes instead of slicing an empty UTF-8 fallback by byte offsets. Reconstructive edits remain manual when their replaced span contains non-UTF-8 data, while safe edits preserve raw data outside the edit span.

Safe-navigation calls are excluded where the pinned Ruby patterns require ordinary sends. This fixes nil-semantics changes in IdentityComparison, ClassEqualityComparison and HashExcept and a false-positive MultipleComparison diagnostic. Four additional explicit runtime controls cover the reproduced cases.

## Positive coverage and reporting

Twenty-two new successful-rewrite programs cover BooleanSymbol, ErbNewArguments, ClassEqualityComparison, HashExcept, RandomWithOffset, SlicingWithRange, UnpackFirst and SafeNavigationWithEmpty. Every program rewrites under all-correction mode; eleven also rewrite in safe mode. Twenty-one preserve original observed behavior, while one BooleanSymbol case records the intended shared Symbol-to-boolean change. Each of the eight cops has at least one behavior-preserving positive program.

ERB's legacy positional signature is no longer accepted by the installed ERB 6.0 library. Its programs use a small legacy-signature adapter that forwards into real ERB rendering, verifying argument mapping, trim mode, output variable and evaluation effects rather than relying on a failing legacy constructor.

The new `runtime_activity` report separates scenario selection from actual positive source rewrites, unchanged negatives/report-only/not-applied cases, and blocked executions. Positive counts require valid original/native Ruby, an actual native source change, completed normal execution and no native-only runtime regression. Preserved behavior, shared reference changes and failures are separate outcomes. Missing, skipped or malformed execution artifacts cannot produce successful-positive coverage.

Raw-byte regressions are permanent Rust byte-vector tests, complementing the UTF-8 JSON corpus. Generated programs remain unauthorized for runtime execution. Cache rules revision is 46.

## Final verification

- Full static differential audit: 5,311 selected examples and 5,271 compilable comparisons across all 98 additions, zero new failures. Runtime execution was verified separately.
- Fresh full runtime audit: all 181 trusted scenarios across 49 cops, zero native-only failures; successful positive rewrites for all 48 source-rewriting cops. There are 123 positive scenarios, with 88 safe-mode and 123 all-mode successful rewrites. The remaining scenario-only cop is diagnostic-only DuplicateCaseCondition.
- 827 Rust tests pass; four dedicated byte/safe-navigation tests cover 60 mode checks. The final byte tests pass after simplifying raw-span bindings.
- 52 Python integration tests pass. Refactored reporting reproduces both audit summaries exactly from their artifacts.
- 108 final release CLI checks and a separate 24-run native/Ruby raw-byte CLI/runtime matrix pass, including exact correction-byte equality.
- Formatting and ABC checks pass. No new Clippy diagnostic signatures were introduced; the existing backlog remains.

The central corpus now has 3,432 references, plus 30 separate pinned references. Positive source-changing execution is stronger evidence than scenario presence, but remains finite testing rather than proof of arbitrary Ruby correctness.
