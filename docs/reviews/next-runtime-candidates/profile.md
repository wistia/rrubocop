# Next performance candidates on the current branch

Read-only investigation of `3982e39` (implementation `6cdec63`). No implementation edits, builds or broad test reruns. All timing work was sequential, with no other agent builds/heavy scans. The main conclusion is to prioritize shared configuration preparation, then CodeMap membership, over another speculative rewrite of the already-small custom-check budget.

## Current corpus and attribution

| Scope | Wall seconds | Aggregate user CPU seconds | Repeats |
|---|---:|---:|---:|
| Current full suite | 28.746 | 218.526 | 1 |
| Current excluding custom28 | 28.465 | 216.655 | 1 |
| Current cache backend unavailable | 28.432 | 216.345 | 1 |
| Pre-lazy full suite | 28.613 | 217.352 | 1 |

All four runs inspect 14,762 files and report 27 offenses. Full-run **counts**, not fresh complete diagnostic arrays, were retained for this investigation. Custom28 exclusion changes wall time by approximately **0.280s / 0.98%** and user CPU by **1.87s / 0.86%** in the one current pair. Most runtime is outside the custom-check increment; no individual custom rule dominates these measurements.

Per-binary/per-group isolated valid cache directories hold fewer than the 20,000-entry retention cap. Current versus pre-lazy full runs are 28.746s versus 28.613s (+0.46%); the 632-file three-repeat sample is 2.480s versus 2.478s (+0.10%). These do not establish that the prior +1.2% regression disappeared, nor pinpoint a losing change. Small differences remain inconclusive.

Current sample medians (632 same files, three repeats):

| Scope | Wall seconds | User CPU seconds |
|---|---:|---:|
| sample-all | 2.480 | 9.706 |
| sample-no-custom | 2.447 | 9.568 |
| sample-no-autoload | 2.479 | 9.702 |
| sample-no-rescue | 2.475 | 9.704 |
| sample-no-lazy-indexes | 2.483 | 9.712 |

These matched exclusions cannot be summed into pure per-rule costs: filtering, shared activation and cache/framework work interact. Autoload/rescue/index-family deltas are small/noisy here and provide no trustworthy individual ranking.

## 1. Resolve effective configuration once per file/layer

Source: `src/config/resolved_cop_cfg.rs:61`, `resolved_effective.rs:34-41`, `linter/engine/mod.rs::select_active/run_non_syntax`.

Every selected cop under a nested override calls `effective_config_for_file`: clone the full ResolvedConfig, clone its layer snapshot, merge the override, copy maps again, then extract one CopConfig. It repeats for every cop. Ordinary files also repeatedly clone/inject CopConfig and the non-syntax phase clones active configs again.

A corrected synthetic control uses two identical small Ruby files and a root config explicitly mentioning all registered cops. The first argument is always a root file, ensuring root discovery; the second is another root file or a file under a nested `.rubocop.yml` that only repeats the same IndentationWidth=2 value. Thus the nested control actually exercises the override path. Three alternating repeats:

| Requested cops | Both files at root | One file under equivalent nested override |
|---:|---:|---:|
| 1 | 0.035s | 0.038s |
| 100 | 0.036s | 0.207s |
| 300 | 0.039s | 0.570s |

The extra ~0.171s at 100 requested cops and ~0.531s at 300 on **one override file** strongly supports the source-backed repeated-merge cost. Counts are requested, not instrumented active callback counts. Both controls inspect two same-content files; expected findings/counts match, but paths differ.

Real Wistia currently has only one nested config (`dev/spec/.rubocop.yml`), covering 16 Ruby files. Nevertheless it is expensive: with a project-root Gemfile sentinel first (17 files total), normal nested resolution takes **4.080s**, while explicit root config plus excluding the five cops disabled by that nested file takes **0.989s** (three alternating repeats). Complete offense arrays are identically empty and file counts match. This is **not a pure clone-function profile or proof that every internal cop-config/activation field is identical**: explicit config, directory discovery, final preparation and filtering paths differ. The nested YAML only disables those five cops; the synthetic control independently confirms the clone path. Do not subtract the entire 3.09s from a full-run forecast without implementing and validating the change.

Recommendation: cache/prepare one effective ResolvedConfig per distinct directory layer/lint operation, then borrow/pre-resolve CopConfigs rather than cloning full maps per cop. Preserve nested precedence, project mentions, AllCops/global and sibling injections, Includes/Excludes, versions, disabled-by-default and per-run lifetime. This is the highest-confidence candidate with actual-codebase evidence.

## 2. Replace linear CodeMap range membership

`src/parse/codemap.rs::covers` linearly scans sorted non-code ranges for every query. Layout/SpaceAfterComma calls it per comma; EmptyLines/IndentationWidth and several other layout cops also query it frequently. A compliant literal array with N strings and commas, two repeats each, cache backend unavailable:

| Strings/commas | Syntax seconds | SpaceAfterComma seconds |
|---:|---:|---:|
| 1000 | 0.020 | 0.022 |
| 4000 | 0.037 | 0.049 |
| 8000 | 0.058 | 0.109 |

At 8,000 ranges the cop adds ~0.051s above Syntax; doubling 4,000 to 8,000 approximately quadruples the incremental cost. Source and measurements support candidate-dependent O(query-count × range-count) growth. This is a framework/layout opportunity, not a custom cop, and its total production share is not established here.

Recommendation: merge overlapping/adjacent ranges once, then binary-search membership (or use an order-aware cursor where appropriate). Searching only the last starting range without merging can be wrong for overlapping ranges; preserve exact union, empty/end-exclusive ranges, heredocs/interpolation and byte offsets. Benchmark on real string/comment-heavy files before claiming a corpus saving.

## 3. Cache writes/pruning and measurement isolation

`--cache false` disables reads but still writes a redb transaction per file. Startup pruning parses every stored row; MAX_ENTRIES is 20,000 versus 14,762 files per full corpus. Alternating revisions or filter groups creates different keys and can push a shared cache over the retention cap repeatedly. This is a plausible confound for previous tiny runtime/RSS differences, **not a proven explanation**.

Two-repeat current sample medians:

| Cache state, reads off | Wall seconds | User CPU seconds |
|---|---:|---:|
| shared | 2.527 | 9.758 |
| isolated | 2.487 | 9.733 |
| unavailable | 2.456 | 9.650 |

A full isolated-valid-cache run is 28.746s versus 28.432s with backend unavailable, approximately 0.314s difference in one sequential pair. The unavailable experiment points the cache-dir environment variable at an explicitly created temporary regular file; Cache::open safely returns None. It is a benchmark control, **not a product switch**, permissions bypass or a change to the actual cache. All runs still pass `--cache false`.

Recommendation: measure batching writes and conditional/cheaper pruning, and consider an explicit no-read/no-write mode if product semantics permit. Current measured contribution is modest, so prioritize configuration/CodeMap first. Use isolated cache directories in future timing comparisons.

## 4. Small, concrete custom-rule negatives

### Non-Redis setters should not initialize the transaction index

`NoSetWithoutExpiration::redis_block` currently initializes a whole-file Transactions index for a non-Redis `.set` lacking expiration, even if its immediate parent cannot be a transaction body. One `object.set("key","value")` followed by unrelated methods:

| Background methods | Syntax seconds | NoSetWithoutExpiration seconds |
|---:|---:|---:|
| 1000 | 0.031 | 0.036 |
| 10000 | 0.184 | 0.220 |

At 10,000 unrelated methods, unnecessary first-use indexing adds ~0.036s. Real generic `.set` calls exist in brand-kit/context/services code, but their total runtime share is unmeasured and probably modest. Reject structurally impossible transaction membership **before** requesting the source cache; preserve exact body/parameter/whole-text rules for candidates that remain. This is a small low-risk follow-up, not another NoSetex reparsing bug.

### Avoid constant-reference parent lookup before definition rejection

The autoload cop subscribes to all constants/scope-resolution nodes; `definition()` calls `Node.parent()` on ordinary references to recognize multiple-assignment targets. Repeated root-based lookup remains outside the bounded-name optimization. A class containing `consume(Foo::Bar)` repeatedly:

| References | Syntax seconds | Autoload cop seconds |
|---:|---:|---:|
| 500 | 0.021 | 0.023 |
| 1000 | 0.026 | 0.031 |
| 2000 | 0.036 | 0.048 |

These are small deltas; do not assign a precise asymptotic exponent from them. Consider dispatching multiple-assignment container nodes once and examining their targets rather than subscribing to every reference. Preserve offense locations/count/order for destructured/qualified assignments. Unconditional full-file parent maps could cost more than this guard on ordinary files.

## Low priority: directive parsing

A Syntax-only matched sample (632 files, no redundant-disable audit cop selected) takes **0.929s normally versus 0.920s with --ignore-disable-comments**, three repeats and identical empty offense arrays. This estimates a small filtering/preparation upper-bound under that scope, not a large hotspot. Normal full-suite flag comparisons would change behavior and were intentionally avoided.

## Evidence quality and rejected controls

- Valid curated evidence contains 102 timing rows; source inputs are nine generated variants, all compiled on Ruby 3.4.9 without execution. Rust Syntax controls also report no errors.
- Initial `config:*` single-file controls (24 rows) loaded the child config as top-level and did not exercise directory overrides. They are excluded from conclusions/evidence's valid runs. Corrected `config-fixed:*` pairs force root discovery.
- Naive real dev/spec-only and appended-sentinel controls failed inherited standard-gem resolution because the first root selected the child config without a Gemfile.lock there. No application directory was modified. The successful control puts Gemfile first and uses explicit root config with the exact nested disable list.
- All timings are small-repeat, same-host directional evidence; fresh full scans retained counts rather than complete offense arrays. No sampled CPU profile was available under perf_event_paranoid=4, and no bypass was attempted.
- Raw uncurated rows: `/tmp/rr-next-candidates-profile.json`; curated provenance, medians, validity/rejection notes and all valid rows: `/tmp/rr-next-candidates-profile-final.json`.

Recommended order: **effective configuration preparation → CodeMap membership → cache write/prune policy → small Redis/autoload dispatch negatives**. Further serializer/policy index architecture work has little measured current-corpus headroom compared with the framework work.
