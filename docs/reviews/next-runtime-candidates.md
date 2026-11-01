# Next runtime optimization candidates

Read-only audit of `3982e39` / implementation `6cdec63`. Three agents investigated the current release: two source reviewers and one profiler. No implementation changes or builds were made. The goal is identifying the next improvements relevant to the actual codebase after the sparse-input optimizations.

## Recommended order

| Priority | Candidate | Evidence and rationale |
|---|---|---|
| 1 | Prepare effective/cop configurations once per invocation and directory group | Current code deep-clones/injects each active CopConfig per source, clones it again between phases, and rebuilds the whole effective config per cop under nested overrides. Existing precompute_cop_configs has no callers. Corrected controls strongly confirm this cost. |
| 2 | CodeMap union plus binary-search membership | Ranges are sorted but every membership query scans all ranges. Layout checks query at commas/lines, including compliant code. Literal-heavy probes show growing overhead. |
| 3 | Stop autoload callbacks on ordinary constant references | Every constant/scope reference performs a root-searching parent lookup merely to reject it. Subscribe to multiple-assignment target containers instead, retaining class/module/direct-assignment checks. This is the strongest remaining custom-only candidate. |
| 4 | Avoid ordinary-set transaction indexing | NoSetWithoutExpiration can build a full-file transaction index for a non-Redis setter in a file with no transaction. A once-per-source exact supported-method marker can safely reject those files. |
| 5 | Batch result-cache writes / improve pruning | Uncached analysis still writes one database transaction per source. Measured full-run cache overhead is modest; preserve warm-cache behavior and current cache-false semantics. |
| 6 | Share directive parsing; prepare dispatch/path forms | Duplicated source/config setup is real, but measured directive-only savings are small. Bundle low-risk cleanup after the larger candidates. |

## Configuration evidence

Actual application config has one nested override directory, `dev/spec`, with 16 Ruby files. A control includes those files plus the root Gemfile sentinel so both commands resolve the same project. Normal nested resolution takes a median **4.080s** versus **0.989s** for an explicit root config with the five exact nested-disabled cops excluded. Three alternating repeats inspect the same 17 files and produce identical complete offense arrays.

This is not an implemented speedup or pure function profile: the control changes configuration/selection paths, and identical zero findings do not alone prove every option is identical. It establishes a credible optimization target when combined with the independent synthetic control: two identical-content files under the same common root, with one file placed under a semantically equivalent nested layer. One requested cop takes approximately **0.035s root / 0.038s nested**; 100 requested cops **0.036s / 0.207s**; 300 requested cops **0.039s / 0.570s**. Three repeats per scope show the multiplication with enabled cops.

The implementation path is explicit: cop_config_for_file calls effective_config_for_file per enabled cop; that clones the entire resolved configuration and layer snapshot before merging. Then run_non_syntax clones active CopConfigs again. Prepare immutable registry-indexed root/effective-directory configs once per lint invocation and borrow/share them during checking. Preserve all global/sibling injection, path filtering, directory enablement, target versions and refresh between MCP requests. The unused precompute_cop_configs helper is a starting point, not a complete solution by itself.

## CodeMap evidence

| Compliant literal strings | Syntax-only seconds | SpaceAfterComma seconds |
|---:|---:|---:|
| 1,000 | 0.020 | 0.022 |
| 4,000 | 0.037 | 0.049 |
| 8,000 | 0.058 | 0.109 |

These totals include parsing, framework activation and map construction; their difference is not exclusively the covers function. Source inspection confirms O(queries × ranges) membership work. Merge the ranges into their exact union and use predecessor lookup, preserving exclusive ends and handling overlaps/empty ranges. Test equivalence against the current predicate at every byte offset. Many existing layout checks benefit; a mutable monotonic cursor is unsuitable as the general API because query order is not universal.

## Remaining custom-only candidates

Autoload constant-reference probes at 500/1,000/2,000 references take roughly **0.023/0.031/0.048s**, versus Syntax controls **0.021/0.026/0.037s**. Registry subscription includes ordinary references that cannot be definitions. Container-based multi-assignment dispatch can avoid those callbacks, but must preserve direct-child target selection, nested destructuring, scoped target ranges/counts/order and existing ignored target shapes. Do not trade the current rejection for another parent query on each reference.

A sparse ordinary non-Redis setter with 10,000 unrelated methods takes approximately **0.220s**, versus Syntax **0.184s**. The transaction index is constructed even when no multi/pipelined call can exist. A per-source negative marker can bypass it, while a marker hit continues to use the exact structural classifier. Do not add per-set Node.parent calls: that can recreate wide/deep root-search costs. Actual eligible application files contain these ordinary setter calls, although their aggregate contribution remains unisolated.

Further serializer strict/broad map splitting or more namespace normalization shortcuts are conditional, lower-priority candidates. Current serializer files are modest, and the recent per-family lazy split is already implemented. No new evidence supports revisiting NoSetex reparsing, sparse rescue memoization or the fixed TreeCursor traversal.

## Whole-codebase context and cache controls

Current release, isolated caches, one matched full pair:

- All checks: **28.746s**, 218.526 user-CPU seconds, 14,762 files, 27 findings.
- Excluding the 28 custom checks: **28.465s**, 216.655 user-CPU seconds, the same file and finding counts.
- Cache backend unavailable: **28.432s**, 216.345 user-CPU seconds, the same file and finding counts.

Fresh full-run controls retained counts, not complete diagnostic arrays; no fresh full-array equality is claimed for those scopes.

The marginal custom-group difference is approximately **0.280s / approximately 1%**; this includes changes in selection/shared setup and is not a precise intrinsic rule cost. It leaves limited evidence for a large whole-codebase win from more custom-only micro-optimizations. The cache pair suggests approximately **0.314s** total overhead under these conditions, so cache batching is worthwhile but not the leading candidate.

Cache-false skips reads but still writes, and startup pruning examines all cached rows. The 20,000-entry cap is smaller than two 14,762-file revision/settings populations. Prior old/new comparisons shared a cache, so pruning history and database state are potential confounds. This pass used per-binary/per-group directories; an intentionally unavailable temporary backend exercises the existing no-cache fallback solely as a diagnostic control, not a recommended product flag.

With isolated caches the current and pre-lazy binaries are almost tied on the repeated 632-file sample (2.480s versus 2.478s). The one full old/new isolated pair is **28.746s current versus 28.613s previous (+0.46%)**. That neither reproduces the exact earlier +1.2% number nor proves it was caused by cache pressure. The previous small runtime/RSS difference remains a measurement rather than an established causal diagnosis.

Syntax-only directive controls take **0.929s normally / 0.920s ignoring disable comments** on the fixed sample. Reusing parsed directives is a small measured cleanup, not a major speedup promise. Normal full-rule ignore-disable comparisons would change findings and are not used for this claim.

## Sources and methodology

- [Framework source audit](next-runtime-candidates/framework.md)
- [Custom-rule source audit](next-runtime-candidates/custom.md)

- [Detailed profile and control qualifications](next-runtime-candidates/profile.md)
- [102 valid timing records, provenance and rejected controls](next-runtime-candidates/measurements.json)

Nine generated Ruby source variants compile with Ruby 3.4.9 without execution. All proposed speedups remain unimplemented. Configuration controls that accidentally loaded a nested file as the root were excluded; failed Gemfile-lock lookup experiments are not timings. No application files were modified.
