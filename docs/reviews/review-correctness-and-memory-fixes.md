# Review correctness and memory fixes

Follow-up to the independent review of `f31620a7dcddf720952c28802b2a8186fd2aa611`.

## Correctness

- CodeMap recognizes tree-sitter's actual `delimited_symbol`, `symbol_array`, and `subshell` containers. Percent-literal punctuation delimiters stay protected, so safe spacing corrections preserve symbol and shell-command values. Interpolated Ruby remains exposed.
- Whole-line orphan-enable removal uses actual next-line/EOF byte boundaries, including CRLF terminators.
- Redundant-disable ranges distinguish originally single-cop directives from block records split out of multi-cop directives.
- Per-cop directive ranges resolve `all`, department, and specific selectors in source order. Suppression, disabled-edit filtering, and redundant-disable attribution share those ranges. Inclusive enable lines and independent trailing disables retain Ruby semantics.

Related reference tests cover comment prefixes, trailing versus block directives, `todo`, repeated specific disables, and department re-enables. The existing policy of preserving untouched CRLF bytes remains: Ruby CLI normalizes rewritten files to LF, while Rust retains their original terminators. The earlier documented `=begin` exception is unchanged.

## Optimizations

- Conflict indexes and duplicate keys borrow original corrections and replacement text; selection moves each payload once. HashMap equality still compares the full replacement, including collisions.
- CorrectionSet construction and engine finalization move accepted edits instead of cloning disposable payloads. Alias reconciliation completes before the move.
- Fully owned correction batches return before constructing unused diagnostic candidates.
- Unicode coordinates store continuation-run deltas when compact and fall back to the old dense representation when smaller. ASCII fast paths remain. Arbitrary invalid-byte prefix-count conventions are preserved.
- Non-interpolating literal forms skip descendant searches for interpolation.
- Static-symbol labels check for the necessary colon suffix before looking up a parent node.

Cache rules revision advances to 31. No application source files were modified.

## Validation

Implementation: `91cfe88ea38f4f1940d9636dc1205742e6da9ef3`.

- 761 tests pass across all targets. The final cache-type alias also passes Clippy type checking with no new diagnostics: 224 preexisting diagnostics remain.
- [Pinned reference](../../tests/fixtures/engine/review-fixes-reference.json): 30 inputs through Off/Safe/All, yielding 90 matching offense-metadata results, including locations, highlight lengths, messages, correctability, and correction status. Eighty-six corrected-byte results match Ruby exactly; four explicitly preserve CRLF with documented reasons.
- 200,000 randomized selections match the actual prior selector snapshot, including grouped edits and aliases. Sparse/dense coordinate tests cover 4,096 arbitrary byte vectors at every offset/column, plus Unicode-heavy fallback.
- An independent directive-range model matches bundled Ruby on 1,000 randomized eight-switch sequences; the 90 engine comparisons above exercise actual Rust behavior.
- 67 earlier CLI inspection probes and 21 earlier correction probes retain identical complete diagnostics and corrected bytes.
- Changed Rust formatting, diff checks, and ABC pass: zero function/module complexity offenses, 80 used-once advisories, zero never-used warnings. No lint suppression or threshold changes were added.

## Measurements

Same release profile and fixed corpus; sequential before/after processes, warm filesystem, three alternating repetitions per binary for targeted scopes and sample. Full runs use two per binary in before/after/after/before order. No builds or agent scans ran during timings. Isolated per-binary/per-scope cache directories avoid revision population/pruning effects; `--cache false` still writes. Synthetic probes use an unavailable backend. Correction sources reset before each invocation.

| Scope | Before median (s) | After median (s) | Before RSS (KiB) | After RSS (KiB) |
| --- | ---: | ---: | ---: | ---: |
| percent-4000 | 0.023433 | 0.022590 | 14196 | 14584 |
| percent-64000 | 0.166125 | 0.150294 | 39028 | 39292 |
| ascii-1m | 0.037914 | 0.038601 | 14324 | 15036 |
| unicode-sparse-1m | 0.042888 | 0.039879 | 22264 | 15028 |
| unicode-dense-250k | 0.025122 | 0.026284 | 16372 | 16948 |
| correct-8000 | 0.288315 | 0.281166 | 34420 | 32060 |
| sample | 1.712839 | 1.719031 | 75392 | 75276 |
| full | 20.800648 | 20.519382 | 137384 | 140646 |

All paired benchmark offense arrays and corrected source bytes match. Full scans inspect 14,762 files and retain 27 offenses. The sample and full-scan timings remain close; this pass does not establish a material whole-codebase speedup. Compare binaries within this batch, rather than against older reports collected under different load.

The percent scopes contain non-interpolating `%w` words, selecting SpaceAfterComma. Unicode scopes select NoSleep on one long comment-bearing line: sparse is one million ASCII characters plus one `é`; dense is 250,000 emoji. ASCII is the same-length control. These are targeted scaling probes, not typical source sizes. The large sparse-coordinate RSS reduction holds in the real CLI; the small dense/ASCII differences include process-startup and parser variation.

Separately, three alternating standalone runs use the actual selector/coordinate source snapshots. The 32 disjoint 1 MiB replacement probe drops median RSS from 100,136 to 34,596 KiB. The mostly ASCII coordinate probe drops from 10,980 to 3,616 KiB; the Unicode-heavy probe remains 7,908 KiB in both. These isolate the relevant structures and do not represent whole-engine or application memory use. Extra moves in CorrectionSet/finalization and the two small negative guards have no isolated timing attribution.

[Raw timings, source/binary hashes, standalone memory measurements, comparison records, and runners](review-correctness-and-memory-measurements.json).
