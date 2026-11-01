# Further performance audit of the 28 custom checks

Read-only audit of `5b95a2e` (optimized implementation `3f5fca0`), following the [previous optimization work](port-fixes-and-optimizations.md). No implementation changes are included. **Findings below remain unfixed.** Two static reviewers covered all 28 app-specific rules; one profiler measured the existing release binary, matched corpus controls and candidate-heavy inputs. Already-fixed NoSetex reparsing and eager edit rendering are not being proposed again.

## Priority 1: repair the shared streaming traversal regression

`src/cop/ported/context/walk.rs` advances using `Node::next_named_sibling()` and `Node::parent()`. These operations are not constant-time pointer steps. In the pinned tree-sitter 0.26.13 implementation, `src/node.c:547` starts parent lookup at the tree root, and `:251` implements sibling lookup with another parent lookup and child enumeration. Repeating these operations amplifies both nesting depth and wide sibling lists.

This was introduced by our previous streaming-walker change. A preserved pre-change binary on the same synthetic source/configuration confirms the regression: 100 nested rescue handlers take roughly **0.784s now versus 0.054s before**. At 200 handlers the current cop takes about **6.0s**, while Syntax is about **0.044s**. The 400-handler cop run was stopped after more than 30 seconds; that is a timeout, not a completed timing.

Use a stateful TreeCursor traversal or an explicit child-iterator stack to retain position without root-based navigation. Preserve named-node preorder, root inclusion, comments, subtree bounds and short-circuit iteration. Existing traversal equivalence tests and the 2,054 reference cases are acceptance gates. Re-measure before adding more complicated per-cop caches: this shared repair may remove much of the observed cost in several checks.

## Other concrete opportunities

| Priority | Area | Remaining work | Recommendation |
|---|---|---|---|
| High | Three serializer checks | Search an owner/parent subtree for each attribute, even when no matching method exists | Index method names/positions once; preserve the different strict-owner and broad-parent matching policies |
| High | ExplicitSidekiqQueue | Collects all class calls and repeatedly resolves ownership before deciding whether the class is a worker or already declares a queue | Filter to include/sidekiq_options/use_default_queue! first; stream, short-circuit and prune/index nested owners without changing method/block semantics |
| Medium | RescueMustReportError | Nested handlers repeatedly inspect overlapping subtrees | After fixing traversal, consider lazy reporter summaries/range queries; preserve bare raise/local binding, safe navigation and existing descendant semantics |
| Medium | NoSetWithoutExpiration | Each direct transaction set enumerates the whole body and lowercases the enclosing transaction before checking expiration | Check expiration first; stop counting after two non-comment children; memoize exact transaction qualification/parameter |
| Conditional | Autoload names and filesystem | Reconstructs namespace prefixes per definition; project roots/initializer reads/require results still repeat across files | Share namespace prefixes and a lint-operation project snapshot, with explicit filesystem/config invalidation |
| Low | Bare identifier routing and lexical index | Sleep/foreign-key/timer callbacks see every identifier; model index is seeded even without a matching call | Measure routing gates and demand-driven index construction before extending the framework |

These are diagnostic-only checks except the shared framework's broader consumers. No recommendation requires weakening correction safety, skipping correctability analysis, retiring checks or changing pinned rule semantics.

Redis details matter: raw named-child count includes comments whereas the current helper excludes them. A replacement must count at most two **non-comment** named children. Keep the current whole-transaction text predicate and immediate-body scope rules; narrowing to only a receiver name would be a behavioral change. Expiration-first evaluation can reuse the existing pure helper and safely avoid transaction analysis for compliant calls.

## Current corpus impact

A sequential matched full-codebase pair inspected 14,762 files: **31.428s with all checks versus 31.002s excluding the 28 custom checks**, a difference of approximately **0.426s / 1.37%**. Three repeated fixed 632-file sample runs give medians **2.567s versus 2.487s**. This is evidence of a small aggregate custom-rule cost on the current clean corpus, not a precise per-rule accounting or a promised CI saving. Standalone --only timings include startup, configuration and shared activation and must not be summed.

The current eligible serializers are modest; a crude attribute-count × method-count maximum was 64. Artificial files with hundreds of declarations or deeply nested classes/handlers demonstrate scaling risks, not typical application latency. Prioritize the shared walker regression and small safe guard-order/filtering changes; measure their real impact before undertaking broader cache architecture work.

## Supporting evidence

- [Call and serializer static audit](custom-rule-performance-next/calls.md), covering 18 rules.
- [Scope, runtime and autoload static audit](custom-rule-performance-next/scopes.md), covering the other 10 rules.

- [Detailed measured profile](custom-rule-performance-next/profile.md), including scaling tables and controls.
- [173 timing records and provenance](custom-rule-performance-next/measurements.json), including binary hashes, all-28 inventory, source/corpus revisions and preserved/current diagnostic equality.

All 37 generated variants compile with Ruby 3.4.9 without execution. Static complexity claims count logical visits; actual wall-time growth is amplified by the shared navigation issue, so do not assume constant cost per visited node. There were no builds or broad test reruns during this audit; the only repository changes are these review notes.
