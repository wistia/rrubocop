# Further custom-rule performance audit: scopes, autoload and runtime checks

Read-only inspection of `5b95a2ebbccd9013e6484490131bef005c89ec17` (implementation `3f5fca0`). No production edits, builds, tests or benchmark runs. These are **remaining** opportunities after per-source caches, streaming walks, call-name dispatch and lazy correction payloads were implemented. Dynamic measurements belong to the companion profiling agent; no speedup is claimed here.

Scope: 10 of the 28 app-specific rules: `Cops/NoUndiscoverableAutoLoadedConstants`, `Cops/NoRequireOnAutoLoadedPaths`, `Cops/RescueMustReportError`, `Cops/EnforceNoShowInPolicies`, `Cops/EnforceScopeInPolicies`, `Cops/ExplicitSidekiqQueue`, `Cops/NoConstantDefinitionsInSpecs`, `Cops/NoTopLevelSideEffectsInAutoloadedFiles`, `AnyCable/InstanceVars`, `AnyCable/StreamFrom`. The call/serializer auditor covers the other 18.

## Best concrete low-risk candidate: filter and short-circuit queue detection

Locations: `src/cop/ported/wistia_scopes/queue.rs:10`, `:20-30`, `:33-48`.

`ExplicitSidekiqQueue` still materializes **all** calls belonging to each class before it knows whether the class is a worker or has a queue. Every candidate call walks its ancestors to find its nearest class/module. Ordinary unrelated calls in method bodies incur the walk too. Nested classes are traversed again by each outer class, only to be rejected by nearest-owner checks.

Suggested implementation: consider only `include`, `sidekiq_options` and `use_default_queue!` before doing owner lookup. Accumulate the two relevant booleans in a streaming scan; a known BaseWorker can return at the first valid queue declaration. Prune nested class/module subtrees, or pre-index nearest owner once if multiple class-scanning cops share it. This removes the `Vec` and avoids most ancestry queries without requiring a general framework rewrite.

Expected benefit: class bodies with many ordinary sends, especially already-correct workers whose queue declaration is near the beginning. Nested classes amplify repeated traversal; normal config limits this check to `app/workers/**/*.rb`, so broad corpus impact must be measured rather than assumed.

Parity safeguards: preserve the single-body-statement exclusion at lines 21-25, Nsq exclusions, subclasses ending `::BaseWorker`, inherited-worker/include matching, exact queue pair handling and the current fact that calls inside methods/blocks count when their nearest class/module is the worker. Restricting the scan to direct class statements would silently change semantics. Diagnostic-only: no direct autocorrection risk.

Benchmark suggested to profiler: a queued BaseWorker with its declaration first, followed by 100/1,000/10,000 irrelevant sends; compare same-size Syntax run. Also deeply nested nonworker classes with calls to expose owner-walk amplification.

## Candidate-dependent algorithmic issue: overlapping rescue handler scans

Locations: `src/cop/ported/wistia_runtime/rescue.rs:10`, `:17`, `:34-35`.

Streaming descendants removed allocations but does not amortize overlapping work. Each broad rescue scans its entire handler until it finds a report. N nested rescue handlers with no report traverse successively overlapping subtrees, giving quadratic checker work in the nesting depth. Deep rescue-modifier handlers have the same general shape. Typical small disjoint rescues remain cheap.

Suggested implementation: memoize subtree `contains reporter` summaries, or collect reporter intervals once and query each handler range. Retain a cheap path for a single rescue/early reporter, since an unconditional full-file index can cost more on ordinary files. This is worth a targeted stress measurement before implementation.

Parity safeguards: reporter recognition must remain exactly the current `reports` predicate, including safe navigation, qualified constants, `Rails.error.report`, bare `raise` versus a local variable named `raise`, and the existing descendant semantics across nested method/class/lambda nodes. Do not prune lexical scopes merely because the report would not execute with the outer handler: that would change pinned Ruby behavior. The cached lexical membership helper is already fixed and should be reused. Diagnostic-only.

Suggested stress source: recursively nest `begin; work; rescue; INNER; end` **inside the rescue handler**, ending in a nonreporting call. N=50/100/200/400; all N offenses should remain. Use same-size Syntax control to separate parser nesting from checker growth.

## Cross-file optimization, beyond current per-source filesystem caching

Locations:

- `src/cop/ported/difficult/autoload.rs:12,31-35` initializes once **per source**.
- `src/cop/ported/difficult/autoload/location.rs:39-49` repeats ancestor marker stats for each eligible source.
- `src/cop/ported/difficult/autoload/inflections.rs:15-25` reads initializer contents before every global cache lookup; byte comparison and cache access happen under one global mutex. Parsed acronyms are already reused. The current Wistia initializer is only 1,219 bytes, so this is primarily repeated open/read/stat and synchronization work, not large-byte parsing.
- `src/cop/ported/wistia_runtime/autoload.rs:14,37-58` memoizes roots and require existence inside each `SourceFile`; identical requires in many files still rediscover roots, enumerate app directories and stat the same candidates per file.

Suggested implementation: a lint-operation/project context shared across source snapshots for project roots, require-root lists, initializer snapshot and `(root, required)` results. Resolve config/root-policy identity before sharing. Share the parsed snapshot through an Arc; refresh at each new lint operation or explicit filesystem epoch. This is different from the completed per-source caching and is the most plausible opportunity spanning many real files, but needs syscall/corpus evidence.

Safety: preserve Gemfile-first resolution for undiscoverable constants versus nearest `.rubocop.yml` for require checks; preserve concern/STI roots, nested configs, multi-project runs and initializer content invalidation. Do not use permanent negative caches or mtime/size alone: existing tests require same-size initializer edits and newly-created/deleted files to be observed in a later source/lint snapshot. Do not replace existence checks with only the linter's discovered-file list: lint excludes can hide files that `require` still loads. A one-operation snapshot policy should be explicit for watch/MCP use.

Suggested benchmark: many small eligible source files, each defining one class and requiring the same missing/existing path; vary number of app roots. A syscall count would help distinguish root/initializer work from parse/config overhead. No build or syscall tool use was attempted in this static audit.

## Namespace reconstruction remains repeated despite cached file expectations

Location: `src/cop/ported/difficult/autoload/names.rs:10-24`, `:36-47`, `:50-100`.

Expected filename and acronyms are cached, but each definition still collects and reverses every ancestor, reconstructs namespace parts and allocates a qualified actual-name string. N definitions at depth D cost O(N*D) ancestor visits, with additional string-copy cost. Nested modules or many constants in the same deep namespace repeat identical prefix work.

Suggested implementation: memoize lexical namespace prefixes at scope boundaries, or traverse once with a namespace stack and compare expected/actual segments without building a complete string for every definition. Prefix memoization alone does not remove the cost of materializing very long qualified names; avoid claiming linear total work if full strings are still built per node.

Safety: the current namespace rules intentionally account for explicit scopes, Object prefixes, Class.new/Module.new assignments, class_eval, singleton classes and unsupported/dynamic block contexts. An optimization must preserve these cases and literal acronym normalization; generic nearest-class lookup would be wrong. Diagnostic-only. Prioritize only if generated/deep namespace stress is meaningfully expensive; ordinary nesting is shallow.

## Smaller opportunities, probably lower value

- `Cops/EnforceScopeInPolicies` performs two subtree scans (scope subclass and disable_scoping) at `wistia_scopes/policies.rs:36-42`; `EnforceNoShowInPolicies` rescans the same policy for each show? at `:18`. A shared per-policy summary can avoid repeat work across both checks. Preserve the exact boolean inequality/exclusivity test and existing descendant semantics. Normal configuration limits these rules to a small policy directory, so likely small corpus impact.
- `NoConstantDefinitionsInSpecs` (`wistia_scopes/constants.rs:8`) and `AnyCable/InstanceVars` (`wistia_runtime/anycable.rs:5`) repeat ancestor predicates per matching node. A shared nearest-context index could help unusually deep files with many constants/ivars, but standalone caching may cost more than these short walks. No recommendation to implement absent measurement.
- `AnyCable/StreamFrom` extracts arguments multiple times (`wistia_runtime/anycable.rs:19-28,36-40,58-64`): callback, valid first argument, then option pairs. A single borrowed argument view could reduce small allocations, but narrow channel path filtering and small argument lists make it a micro-optimization.
- `NoTopLevelSideEffectsInAutoloadedFiles` is registered only for `program` and recursively visits conditional definitions once. No clear repeated expensive work was found. Replacing its bounded statement vectors is low priority.

## Suggested order

1. Measure queue early-exit and overlapping-rescue scaling; implement if their remaining cost is meaningful.
2. Measure per-operation filesystem sharing on a realistic many-file corpus. This may matter more in actual CI than artificial deep nesting.
3. Namespace/policy summaries if candidate-heavy inputs justify the complexity.
4. Leave small ancestry/argument allocations alone unless profiling promotes them.

No rules should be removed for performance on the evidence of this audit. The prior NoSetex reparsing problem, method-name gates, eager correction rendering and allocating generic descendant recursion have already been fixed; they are not new findings.
