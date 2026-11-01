# Custom-rule performance fixes

Implementation `91ccb7c` addresses every optimization category in the [28-rule follow-up audit](custom-rule-performance-next.md), based on `3e0c2c1`. This includes the confirmed traversal regression and the conditional/lower-priority opportunities, with the existing rule semantics retained.

## Further specialization

[Implementation 6cdec63](autoload-lazy-index-optimization.md) further bounds autoload names and splits indexes into demand-driven components. It improves targeted sparse/deep cases, with a small measured full-codebase runtime/memory trade-off; the results below remain the historical 91ccb7c comparison.

## Changes

| Area | Implementation |
|---|---|
| Shared descendant traversal | Stateful TreeCursor iteration replaces root-searching parent/sibling navigation. Named preorder, comments, root inclusion, subtree boundaries and short-circuit behavior remain covered by equivalence tests. |
| Serializer methods | One source-cached topology/method index preserves the separate strict-owner and direct-parent-subtree policies. Stable byte-range/kind keys work across repeated parses of the same source. |
| Queue detection | Only relevant calls update class-owned summaries; explicit queued BaseWorkers can return early. Nested ownership and the single-statement exclusion remain unchanged. |
| Rescue reporting | Multiple rescues share cached reporter-presence summaries; a single-rescue fast path keeps ordinary files cheap. Existing nested-descendant, safe-navigation and bare-raise/local-binding behavior is preserved. |
| Redis expiration | Explicit expiration is checked first. Remaining transaction metadata is cached per body, including exact comment-filtered statement count, block parameter and whole-text Redis qualification. |
| Policy and ancestor checks | Policy summaries are shared. AnyCable/spec ancestor contexts are memoized on demand rather than unconditionally indexing every file for one cheap candidate. |
| Argument/option allocations | Option pairs stream through TreeCursor iteration; AnyCable reuses one argument view. Capitalized-key detection uses an ASCII fast path with the original Unicode fallback. |
| Bare identifiers | Four finite-name rules route only matching identifiers; other identifier cops and callback registration/correction order are preserved. |
| Lexical index | Exact membership is built only when requested. Immutable model scopes move into Arc ownership once and are shared without copying or reparsing; the seed releases after index construction. |
| Autoload namespace | Inherited namespace states are memoized with shared Arc prefixes, preserving assignments, explicit scopes, singleton classes, class_eval and unsupported dynamic blocks. |
| Cross-file filesystem | An explicit lint-operation context shares root markers, inflections, require roots and existence results across files. Standalone/new lint invocations get fresh snapshots; correction iterations retain the operation but recreate source analysis. |

Unqueried lexical seeds retain immutable model scopes until their source snapshot ends; this avoids rebuilding the model later and trades a bounded lifetime extension for removing unnecessary index construction. Peak memory is measured below.

Filesystem observations are lazy snapshots within one lint operation. Tests cover new/deleted files, same-size initializer changes, separate marker policies, nested/multiple roots and rebinding a source to another operation. Standalone constructors initialize the operation lazily. These changes concern check-execution caches; the existing persistent result-cache dependency policy is unchanged. Measurements use `--cache false`.

Rules revision increases to 27, invalidating old cached lint results.

## Verification

- `cargo test --all-targets`: **693 passed**, zero failures.
- All **2,054 pinned reference cases** remain covered, including actual engine Off/Safe/All checks and iterative correction regressions.
- **57 additional CLI differential cases** have identical complete offense JSON against the preserved pre-change binary. Cases cover nested serializer owners, reopened classes, queue exclusions, nested reporters, Redis/comment boundaries, policy contexts, autoload forms and bare identifier shadowing. All differential Ruby sources compile on Ruby 3.4.9.
- Changed Rust formatting and `git diff --check` pass.
- `abcop --full`: zero function/module ABC offenses; 36 advisory used-once suggestions, zero never-used warnings.
- Clippy has the same **230 pre-existing diagnostic identities**, with none added or removed. No lint suppressions or threshold changes were introduced.
- Separate agents cross-reviewed the walker/routing/model sharing and filesystem lifetime/invalidation changes.

## Controlled measurements

All runs use the same release profile and unchanged application checkout, with result-cache reads disabled and warm filesystem caches. No builds or other agent scans ran during measurement. Full scans use interleaved ABBA order (two per binary); sample scans use three alternating repeats; paired stress cases use two per binary. Times below are medians.

| Scope | Before seconds | After seconds |
|---|---:|---:|

| Full suite, 14,762 files | 33.492 | 29.166 |
| Same 632-file sample | 2.636 | 2.579 |
| 100 nested rescues | 0.807 | 0.037 |
| 200 nested rescues | 5.964 | 0.039 |
| 200 nested nonworker classes | 4.519 | 0.037 |
| 400 attributes + matching methods | 1.567 | 0.051 |
| 400 attribute blocks + matching methods | 2.429 | 0.056 |
| 400 compliant attribute blocks, no matching methods | 3.015 | 0.052 |
| 800 compliant Redis transaction sets | 0.165 | 0.055 |
| 200 matching nested namespace modules | 0.730 | 0.060 |
| 50,000 locals, shared model + NoSetex | 0.504 | 0.506 |
| 50,000 locals, three bare-name checks | 0.432 | 0.426 |

The full-scan median improves **33.49s → 29.17s** in this comparison. Before runs span 31.73–35.26s and after runs 28.88–29.45s, so the exact wall-time percentage is noisy. Aggregate user-CPU medians improve **242.17s → 219.02s (~9.6%)**, supporting a real combined reduction. These include shared-helper benefits outside the custom rules; they are not a pure per-rule cost attribution or a guaranteed CI runtime.

The largest paired stress improvement is **200 rescues: 5.96s → 0.039s (~154×)**. The previously unfinished 400-rescue shape now completes in **0.045s**; the previous audit stopped it after >30 seconds, so no completed baseline time or exact speedup is claimed for that size. The 800-block compliant serializer shape completes in **0.068s**. All paired stress, sample and full-codebase outputs have identical complete offense JSON; the full scan still reports the same 27 known regexp findings.

The model-only/no-candidate and identifier-routing experiments show essentially flat or small wall-time changes at 50,000 locals; no material speedup is claimed for those isolated controls. Unit tests establish that index construction and irrelevant callbacks are skipped. The dramatic input-dependent gains must not be extrapolated to everyday files.

Full-scan peak RSS ranges overlap: **150,588–235,860 KiB before**, **171,224–181,816 KiB after**. Parallel scheduling makes the two-run peaks variable, so no consistent peak-memory reduction is established. The 50,000-local model control is approximately 106,400–106,800 KiB for both binaries; the lazy scope-sharing strategy shows no material peak increase there.

[Raw timing, peak-memory, source/binary provenance and 57 differential cases](custom-rule-fix-measurements.json) are committed with this report. Operation-cache reuse and refresh behavior are verified by the epoch/invalidation unit tests; this audit does not claim an individually isolated syscall saving for the filesystem optimization.
