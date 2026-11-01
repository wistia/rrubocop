# Next custom-check optimization candidates

Read-only source audit of `3982e39` (implementation `6cdec63`). No implementation changes, builds, or heavy measurements. The dedicated profiler owns current dynamic evidence. Rankings below are source-based hypotheses unless a later profile confirms them; the previous sparse-input wins are not being reused as proof of current application impact.

## 1. Stop dispatching autoload checks on ordinary constant references

Sources: `src/cop/ported/difficult.rs:13`; `src/cop/ported/difficult/autoload.rs:8-10,49-58`.

The autoload cop subscribes to every `constant` and `scope_resolution`. Its `definition` rejection path calls `Node.parent()` on each such node merely to determine whether it is a direct target of `left_assignment_list` or `destructured_left_assignment`. All ordinary class-name references and qualified constants pay that lookup before returning no finding. Tree-sitter parent lookup searches from the root, so wide/deep reference-heavy files can amplify this cost even though the checker does no naming work. The previous linked-prefix/bounded-head optimization only affects accepted definitions and does not remove this rejection path.

This is the strongest remaining **custom-rule** candidate by likely occurrence: Wistia enables the cop throughout app/**/* and not_production/**/*, where constant references are common.

Recommendation: keep class/module/direct-assignment callbacks, replace constant/scope-resolution interest with the two multiple-assignment container kinds, and check only their direct constant/scope-resolution children. This removes irrelevant callbacks and root searches instead of adding another full-source index. A larger alternative is a stateful source-phase namespace traversal, but that has more parity risk and should follow only if parent discovery for actual definitions remains material.

Safeguards: preserve per-target finding ranges/counts, direct assignment/operator-assignment handling, nested destructuring, explicit scope targets, and the existing treatment of rest/for targets. Do not recursively process all descendants of a target container: that can add previously ignored targets or duplicate checks. Class/module names already receive their own definition callback. Validate all reference cases and differential multiassignment shapes, including scoped and nested targets. Sent a constant-reference-heavy class probe to the profiler, with Syntax and identifier-call controls.

## 2. Avoid the Redis transaction index for files with only ordinary non-Redis setters

Sources: `src/cop/ported/wistia_runtime/redis.rs:5-9,25-28`; `redis/transactions.rs:7-18,32-40`.

Expiration-first checking is already implemented. The remaining issue is a non-Redis receiver such as `context.set(key, value)` without expiration: it enters `redis_block`, which initializes a transaction index by walking the **entire source**, even when the source has no multi/pipelined transaction at all. Indexing is once per source, so the former quadratic issue is not back; the cost is unnecessary first-use work.

This occurs in real sources: folder_settings/customization_patch.rb, map_form_account_config.rb, turnstile_to_form_migration_helper.rb, fix_copy_media_missing_forms_service.rb, brand_kit/application/{mappings,applicators}.rb, media_mover.rb, models/edit/history/extensions.rb, and two not_production limit setters. The cop is globally enabled. Aggregate magnitude is unmeasured and probably modest, but this is a better real-input candidate than another huge synthetic transaction.

Recommendation: an exact supported-method textual fast negative, computed once per source, can skip the index when neither `multi` nor `pipelined` can exist. Marker presence only permits the existing structural builder; strings/comments may cause extra work, never findings. Alternatively check the immediate context only on the first candidate before deciding whether an index is warranted.

Important: **do not add `Node.parent()` on every set call** as the shortcut. That recreates width/depth root-search costs and can be worse on many-set files than today's one linear builder. Keep passed-parent traversal or one-time markers/structural classification. Preserve the whole-transaction-text Redis predicate, case handling, direct-body membership, first parameter, non-comment >=2-statement rule, nested shadows and expiration/payload distinction. Sent the profiler a sparse ordinary-set file with many unrelated methods; no source changes were requested.

## 3. Specialize serializer method data only if eligible-file profiling promotes it

Sources: `src/cop/ported/custom_index/methods.rs:9-13,54-95`; `custom_index.rs:34-37`; `retained/serializers.rs:21-36`; `wistia_runtime/serializers.rs:61-62`.

Methods/queues/policies already initialize independently. The methods component still builds both a global name→ordinal map and owner→name sets for **every method**, allocating the method-name string twice, even if only strict PreferAttribute checks or only broad AvoidMethodAndBlock is active. A file with one queried attribute and thousands of unrelated methods still receives all method data. Also, plural declarations call `custom_index::index` for each symbol, repeatedly taking the typed source-cache lock and cloning an Arc despite querying the same index.

Possible changes: retain only the requested strict/broad representation, intern/share method-name storage, shard by relevant queried owner/parent, or at least hoist the plural declaration's index handle outside its argument loop. These are different from the completed family-level lazy split.

Priority is conditional: configured API serializers were previously modest (crude max attribute×method count 64), so another builder split could introduce extra traversals when both policies are requested and yield little real-corpus gain. Preserve singleton owner boundaries, reopened classes, method-vs-singleton-method distinction, broad parent-subtree semantics, argument offense order and no-autocorrection behavior. Prefer eligible-file ablations over claiming another synthetic speedup. Sent the profiler the strict-only sparse serializer hypothesis.

## 4. Remaining autoload parent discovery and constant normalization

Sources: `autoload/names/prefixes.rs:12-31`; `autoload/names.rs:24-48`; `autoload/inflections.rs:44-79`; `autoload/names/head.rs:1-18`.

For accepted definitions, parent lookup still uses Node.parent per query and per uncached ancestor. Prefix states are memoized, but finding a node's parent is not free. Assigned constants also run two regex replacements, lowercase, camelization and allocate a normalized name before the bounded final comparison. This happens even when the effective namespace prefix already covers all expected filename segments and the suffix cannot affect compatibility.

After removing ordinary-reference callbacks, measure the remaining actual-definition path. Potential next steps: pass parent/namespace state from a stateful traversal; avoid normalization when a **complete effective prefix after exactly one Object:: strip** already determines compatibility; or use a narrowly proven fast path for common normalized names. Memoizing every string in a new global cache is not recommended without repetition evidence.

Safety: raw `parent.starts_with(expected)` is not valid. Root namespace Object plus assigned suffix loses the initial Object:: during comparison; expected Object does not automatically match an arbitrary root constant. Explicit assignment scopes, acronym precedence, invalid-name placeholders, Unicode casing, empty segments, singleton self full-name embedding and unsupported dynamic contexts must remain exact. The new bounded head itself still uses a fresh String with incremental append; pre-sizing could reduce tiny reallocations but needs an allocation profile before becoming another broad claim.

## Lower priorities / leave alone for now

- Queue/policy builders still visit their source once, but they no longer build unrelated maps. More per-owner laziness could rescan nested owners or repeat work when multiple rules use the same family. Their real path filters are narrow (app/workers and app/policies); no additional large cost is established.
- Sparse rescue memo now scans queried handler subtrees and preserves early exits. Its remaining costs are the per-query typed-cache/Arc lookup, mutex and negative memo entries, plus the cheap raw rescue-keyword prefilter. Removing those without data is unlikely to beat broader framework work. Do not reintroduce full-file summaries simply to remove a mutex; that reverses the sparse-input win.
- AnyCable/spec ancestor memoization still uses parent lookup, but candidate counts/path eligibility are narrower than autoload constant callbacks. No separate index architecture is recommended absent profiling.
- NoSetex's reparsing and eager rendering problems remain fixed; correction feasibility must stay available in diagnostic mode.

## Suggested next action

First measure the autoload reference-rejection path and the ordinary-set transaction-index negative path on current eligible sources. They remove unnecessary work before classification without requiring another cache layer. If custom28 still contributes little aggregate cost, prefer the framework auditor's active-cop/configuration/dispatch/setup candidates rather than spending more complexity on rare custom-rule shapes. No measured whole-codebase saving is claimed in this static report.
