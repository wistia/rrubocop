# Next framework optimization candidates

Read-only source audit of `3982e39` / implementation `6cdec63`. No source edits, builds, tests or timing runs. Dynamic measurements belong to `/root/next_current_profile`. Rankings below are implementation-based hypotheses until paired with that profile. The last sparse-index changes improved target inputs but measured +1.2% full wall time / +4.7% peak RSS; this audit does not causally attribute those differences to a specific implementation change.

## 1. Prepare effective configuration once, then reuse borrowed configs

**Broad current-corpus candidate.** `src/linter/engine/mod.rs:237-277` selects enabled cops per file and calls `cop_config_for_file` separately for every enabled cop. `src/config/resolved_cop_cfg.rs:19-39` deep-clones the CopConfig (including YAML option values and Include/Exclude strings), then re-injects global version strings/target settings and relevant sibling options. `src/linter/engine/mod.rs:194-198` deep-clones every active CopConfig a second time merely to remove `Lint/Syntax` from the next phase list.

`ResolvedConfig::precompute_cop_configs` already exists at `src/config/resolved.rs:158`, but has no callers. Config preparation belongs to an immutable effective configuration / lint operation, not every source file or every autocorrection iteration. The simplest first patch can eliminate the second phase-list clone by using references/filtering Syntax in the relevant phase; a broader patch can reuse registry-indexed prepared config values and borrow them during checking.

Nested overrides make the issue more severe: `cop_config_for_file` calls `effective_config_for_file` (`resolved_effective.rs:34`) for **each cop**, cloning the entire ResolvedConfig, taking/cloning a whole layer snapshot, merging it, and applying options. Wistia currently has one nested config under `dev/spec`, with 16 Ruby files. Small file population does not automatically imply negligible cost because each enabled cop repeats a whole config-map clone.

Safe design: prepare the root plus each effective nested-layer identity once per invocation, retain exact global/sibling injection and nearest-directory semantics, and borrow or cheaply share CopConfigs. Do not cache by cop name alone across different configs, ignore directory overrides, or change enabled/Include/Exclude behavior to make timing improve. Preserve configuration mutation between independent MCP/lint requests by scoping caches to the run. Existing config/override/version and all-mode parity tests are required gates.

Suggested experiment sent to profiler: equivalent root-only vs nested-option-override synthetic projects, holding source bytes and actual active cops fixed, varying active-cop/root-config size. Report current `dev/spec` population alongside stress results. Direct attribution of the redundant second clone needs instrumentation or an implementation comparison, so no speedup is claimed from source inspection alone.

## 2. Make CodeMap membership logarithmic or linear per sweep

**Broad algorithmic candidate affecting existing layout checks.** `src/parse/codemap.rs:12-20` sorts non-code byte ranges but every `covers(offset)` still linearly scans them. `Layout/SpaceAfterComma` calls it for every comma (`space_after_comma.rs:53-55`), `Layout/EmptyLines` for every line (`empty_lines.rs:106`), and indentation/spacing/semicolon checks make further queries. With R ranges and P queried positions, the membership component is O(P*R), including already-correct files rich in string literals/comments.

Merge ranges into their exact union once (including overlaps/touching ranges and empty-range handling), then use predecessor binary search. A monotonic query cursor could be even cheaper for an individual source scanner, but consumers do not universally query in order, so changing the general `covers` method to a mutable single cursor would be unsafe. Keep `[start,end)` semantics and string/heredoc/regexp/interpolation treatment exactly unchanged. A useful regression is equivalence to the old `any` predicate for every byte offset, including overlap boundaries.

Important attribution: `Layout/SpaceAroundOperators` accepts a CodeMap but explicitly ignores it; it is not evidence for membership cost. Other source-phase cops also ignore the map. `run_plain_source_cops` unconditionally builds a CodeMap whenever any source cop is active (`source_phase.rs:121-126`), including the custom OutOfRangeRegexpRef, which ignores `_map` and often returns at its cheap `$1`-style prefilter. A separate `needs_code_map` capability could avoid standalone activation costs, but the normal full suite has real consumers anyway; optimize membership first for corpus relevance.

Suggested experiment sent to profiler: valid compliant literal arrays / many string-bearing lines at increasing sizes, SpaceAfterComma/EmptyLines individually plus matched Syntax controls; use real literal-heavy corpus files where possible. Do not infer custom-rule time by subtracting unmatched framework activations.

## 3. Result-cache write batching and prune policy; also a measurement confound

**Broad uncached-run I/O candidate, not a custom checker.** `--cache false` skips reads but still opens/prunes the cache and writes every file (`linter/mod.rs:113-120`, `parallel.rs:277-341`). `EntryStore::store` (`cache/store.rs:36-57`) serializes each result, obtains a redb write transaction, inserts one row and commits. A single database writer can serialize otherwise parallel file workers even with Durability::None.

`Cache::prune` reads/deserializes every stored entry and sorts/removes old ones at process startup (`store.rs:60-92`). MAX_ENTRIES is 20,000; a 14,762-file corpus under two different rule revisions/settings can exceed it. Alternating old/new binaries in ABBA can therefore produce asymmetric startup pruning and different database page/cache states. This may contribute to small before/after wall/RSS differences. It is a **possible confound**, not an established explanation for the last regression.

First measure with isolated cache directories and a documented backend-unavailable control, preserving identical diagnostics. Profiler was given a no-build method: point RRUBOCOP_CACHE_DIR to an explicitly created temporary regular file so create_dir_all fails and the existing `Cache::open -> None` fallback applies. Label it as an unavailable-backend experiment, not a product flag or normal `--cache false` behavior. Avoid editing the user's real cache or using permission tricks.

If measured material, batch/coalesce cache upserts in the runner or use a dedicated bounded writer queue, flushing at run completion. Preserve best-effort cache semantics, file/settings keys, visibility guarantees used by direct cache APIs/tests, fail-fast behavior and correction-mode noncaching. Do not silently redefine `--cache false`: current tests explicitly require writes despite skipped reads. A write-disable option would be a separate behavior decision.

## 4. Parse disable directives once per source snapshot

Parent reviewer independently identified this broad duplicated work. `apply_directives` (`engine/mod.rs:162-177`) calls `filter_directives` twice; each reparses every source line via `directives::parse`, even when there are zero diagnostics. Correction filtering parses a third time in correction mode (`correction/ownership.rs:46`), and directive-specific cops may parse too.

Cache one directive map per immutable SourceFile, or compute it once and share across filters/audit/correction filtering. The initial filter can return immediately on empty findings; the post-audit filter is only needed when the audit added results. Preserve ignored-disable behavior, qualification, case-insensitivity, malformed directives, block/trailing suppression and correction owner reconciliation. Code text must still be extracted correctly; a naive substring search is at most a safe-negative guard, not a replacement parser.

This runs over the ordinary clean corpus as well as files with offenses. The exact cost is unmeasured here; repeated parsing may be more useful than further custom-index specialization now that those checks are cheap.

## 5. Reuse dispatch plans and per-file path forms

`BatchedWalker::with_corr_ok` (`cop/walker.rs:42-58`) allocates/rebuilds kind, call-name and identifier-name maps and per-key index vectors for each file/pass. `ported/call_names.rs:1-5` additionally linearly searches a 48-entry static table per eligible rule during registration. The existing method/identifier gates are already implemented; this opportunity is caching their static **dispatch plan**, not claiming the gates are absent.

Prepare a plan keyed by the ordered active cop identities, retaining current dispatch order and correction gating. Keep each file's current config references/correction mask separate from static routing data. Nested config/version/include effects can change the active set; do not share plans across mismatched sets or reorder callbacks. As a small independent change, store a Rule's call-name slice during registry construction rather than scanning the table on every walker build.

Path matching also repeats relative path/config-directory discovery for each cop: `filter_path.rs:19-55` computes the same path forms separately for Include and Exclude, and `filter.rs:102` hashes cop names even though selection already has registry indices. Prepare file path forms once and use indexed filters/universal-cop partitions where semantics permit. This is a likely many-small-files opportunity, but no measured gain yet. Prefer a simple config-reuse patch before adding an unbounded plan cache.

## 6. Lower priority: typed-cache lookup and model retention

`SourceFile::cached` routes every query through AnalysisCache (`parse/source/cache.rs:20-35`): a Mutex, TypeId/key hash lookup, Arc clone/downcast, OnceLock lookup and another Arc clone. Hot per-definition namespace and per-identifier membership helpers may benefit from dedicated fields or a borrowed per-check handle, but the previous reparsing issue and eager HashSet build are already fixed. New caches may easily cost more than the small local computation they replace, so require a real allocation/CPU profile before extending this architecture.

The lazy lexical seed retains an Arc to full immutable model scopes until either the first membership index is built or SourceFile drops. It releases the seed upon building the HashSet (`model/local_identifiers.rs:12-19`). This is an intentional memory-lifetime tradeoff, not a leak. The 50,000-local prior control showed no material peak increase. Do not propose switching to name-only membership, repeated reparsing or copying all scopes per query. An owned immutable model context shared through all phases could eventually make scope/index lifetimes explicit, but that is a larger refactor than the stronger candidates above.

## Recommended next order

1. Measure configuration preparation/cloning and CodeMap membership on current corpus-relevant shapes.
2. Isolate cache storage/prune overhead before interpreting another sub-percent full-run change.
3. Implement directive parsing reuse if its measured shared cost merits it.
4. Dispatch-plan/path-form reuse if many-small-file cost remains.
5. Leave cache mutex/Arc micro-optimizations and further sparse-rule specialization until profiling promotes them.

No implementation changes are included; proposed benefits remain hypotheses unless confirmed by the companion dynamic profile.
