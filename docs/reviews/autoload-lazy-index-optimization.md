# Autoload naming and lazy custom indexes

Implementation `6cdec63` follows the complexity-based review after `32099f2`. It targets autoload name construction and first-use indexing overhead, with a targeted improvement for sparse rescue handlers.

## Implementation

- **Autoload names:** persistent namespace segments share their parents rather than cloning ancestor vectors. Each segment retains a bounded leading name; normal definition checks avoid rendering the complete qualified name. The bound is expected filename bytes + 10 (one possible `Object::` prefix plus separator/boundary allowance), rounded up to a complete UTF-8 character. Stripping `Object::` once and symmetric namespace-prefix comparison retain their previous behavior. Singleton-class `self` still renders the full inherited name because that name is embedded in its semantic representation.
- **Lazy index components:** methods, queues and policies have independent once-only builders. A policy lookup does not construct serializer method maps or queue summaries. Serializer topology retains only attribute queries and their required parent ranges, rather than a record for every AST node. Strict lexical-owner and broad direct-parent-subtree lookup remain separate.
- **Queue negative path:** non-BaseWorker classes in files without the literal supported `Sidekiq::Worker` marker skip structural queue indexing. Marker presence only permits the existing structural analysis; strings/comments can cause extra work but cannot create findings. Known BaseWorkers still require queue checking, and the single-statement exclusion remains intact.
- **Sparse rescue handling:** summaries are populated only for queried handlers and visited nested handler roots, with early exit on a reporter. Unrelated methods are not traversed to build a whole-file reporter index. The existing single-rescue fast path, reporter recognition, local-variable/bare-raise distinction, safe navigation and nested-descendant semantics remain unchanged.

Source-cache keys remain stable across repeated parses of the same source. Components may each traverse the tree once when multiple families are actually requested; splitting avoids unneeded maps but does not promise fewer traversals for every file. Parent discovery and rare full singleton-name rendering remain costs. Rules revision increases to 28.

## Verification

- **702 Rust tests pass**, including all 2,054 pinned reference cases and Off/Safe/All engine coverage.
- **63 CLI differential cases** match complete offense JSON against the preserved old binary, including additional Unicode, Object-prefix and nested-singleton forms.
- All 63 differential sources and nine generated stress sources compile with Ruby 3.4.9 without execution.
- Tests verify component initialization isolation, persistent-parent sharing, 1,000-segment bounded heads, Unicode/segment boundary equivalence, namespace replay, and sparse/nested rescue cache behavior.
- Changed Rust formatting and `git diff --check` pass. ABC gate reports zero function/module offenses; 41 advisory used-once suggestions remain and there are no never-used warnings.
- Clippy retains the same 230 pre-existing diagnostic identities, with none added or removed.
- Agents independently reviewed bounded-name correctness, component semantics and sparse rescue behavior.

## Measured results

Same release profile and unchanged application checkout; result-cache reads disabled, filesystem warm, no builds or other scans during timing. Full scans ran in ABBA order, twice per binary; sample scopes repeated three times; each targeted comparison ran twice per binary. All complete diagnostic arrays match, including the 27 existing regexp findings on the full corpus.

| Scope | Before seconds | After seconds |
|---|---:|---:|
| Full suite, 14,762 files | 28.628 | 28.978 |
| Same 632-file sample | 2.493 | 2.543 |
| 200 nested modules | 0.081 | 0.056 |
| 600 nested modules | 0.233 | 0.213 |
| 5,000 constants in one namespace | 0.087 | 0.090 |
| One policy query + 15,000 unrelated calls | 0.233 | 0.203 |
| Nonworker + 15,000 unrelated calls | 0.233 | 0.185 |
| One attribute + 10,000 background methods | 0.295 | 0.264 |
| Two small handlers + 10,000 unrelated methods | 0.243 | 0.225 |
| 800 attribute blocks and matching methods | 0.070 | 0.070 |
| 200 nested rescues | 0.037 | 0.040 |

The useful gains are targeted: sparse queue **~21%**, policy **~13%**, serializer **~10%**, and rescue **~7%** faster; nested namespace cases improve **~9–31%**. Dense serializer, wide-constant and already-fast nested-rescue cases show no additional speedup.

**There is no end-to-end win demonstrated on the current corpus:** full-scan medians are **28.628s → 28.978s (+1.2%)**, and user CPU **217.970s → 220.307s (+1.1%)**. Sample wall time also rises about 2%. These are small two/three-repeat comparisons, not confidence intervals; the changes are retained for their targeted scaling and allocation reductions, not presented as a faster CI run.

## Peak memory

| Targeted scope | Before KiB | After KiB |
|---|---:|---:|
| 600 nested modules | 33490 | 22750 |
| Sparse policy | 77520 | 48576 |
| Sparse nonworker | 77524 | 48572 |
| Sparse serializer | 81364 | 54582 |
| Sparse rescue handlers | 50898 | 52280 |
| Full codebase | 149178 | 156118 |

Namespace and sparse policy/queue/serializer peaks decrease roughly **32–37%**. Sparse-rescue and full-codebase peaks are slightly higher; full medians increase about **4.7%** in this comparison. Peak RSS includes the process, parser, configuration and result-cache writes, rather than isolated index heap usage. No general memory reduction is claimed for the whole application.

[All 46 timing/memory runs, source/binary provenance and 63 differential cases](autoload-lazy-index-measurements.json) are committed with this report. Different stress inputs are not a per-rule ranking of the production codebase.
