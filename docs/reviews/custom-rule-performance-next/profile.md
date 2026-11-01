# Further performance audit: 28 custom checks

Read-only audit of `5b95a2e` (implementation `3f5fca0`) using the existing optimized release binary. No implementation edits, builds, or broad test reruns. Scope is all 28 app-specific Cops/Custom/Serializers/AnyCable additions, not all 98 general/custom ports.

## Main finding

**First replace the new shared descendant iterator with a TreeCursor-based preorder walk.** The last optimization replaced allocating subtree vectors with `Node.parent` / `next_named_sibling` traversal. Tree-sitter parent lookup restarts at the tree root; sibling lookup invokes parent lookup and child enumeration. These API costs amplify deep/repeated walks. Preserved-binary comparisons confirm a performance regression, not merely an unmeasured suggestion.

For exactly the same 100 nested rescues/config, the old release takes 0.054s and current release 0.784s (~14.5x slower). Current 200 nested rescues take ~6.0s versus Syntax-only ~0.044s. The 400 case was explicitly stopped after exceeding 30 seconds; this is a lower bound/aborted run, **not a completed timing**. Parent/sibling cost plus repeated handler walks produces worse growth than a simple quadratic label: doubling 100 to 200 increased current total time ~7.65x. Do not assert a precise asymptotic exponent from these few samples.

A bounded TreeCursor traversal should retain streaming/short-circuit benefits without repeated root searches; preserve named preorder, root inclusion, comments, and subtree limits. Benchmark it independently before crediting per-rule indexing savings. The preserved binary is `/tmp/rrubocop-before-optimization-d8470e3`; same-source, same-config diagnostic comparison for 100 nested rescues, 100 serializer attributes, and 100 serializer blocks found identical complete offense JSON arrays (evidence below).

## Current corpus impact

| Scope | All rules wall s | Except custom28 wall s | Increment |
|---|---:|---:|---:|
| 632-file fixed sample, 3 repeats | 2.567 | 2.487 | 0.080s / 3.22% |
| Full 14,762 files, 1 sequential pair | 31.428 | 31.002 | 0.426s / 1.37% |

Both full runs report 27 offenses; both sample scopes report zero. Aggregate child user CPU on the full pair is 238.956s vs 235.988s (~2.97 CPU-seconds / 1.26%). Sample user-CPU medians are 10.655s vs 10.505s. Custom28-only sample median is 0.998s / 1.619 user CPU, but cannot be added/subtracted as pure rule cost: it retains its own Includes/Excludes/activation and framework/parser overhead. No dominant real-corpus custom cop is established by these ablations. The full pair is directional evidence, not a confidence interval.

The application's current eligible serializer files are small: a crude symbol/def text count in the configured `app/serializers/api/v1` and `api/modern` trees has maximum product 64 (16 attribute mentions and 4 defs in motion_kit_serializer). That is context, not a full AST candidate census. Large stress cases below demonstrate pathological behavior rather than promised CI savings. The clean-corpus aggregate increment leaves limited total headroom for these checks.

## Confirmed targeted opportunities

### 1. Serializer method indexes (after correcting the shared walker)

`retained/serializers.rs::matching_method` traverses the owning class for every symbol attribute, including each argument of plural attributes. `wistia_runtime/serializers.rs::matching_method` traverses the Ruby parent for every attribute block. A class with N attributes and N methods repeats scope scans. Methods are appended after declarations; every declared attribute matches in these primary probes.

| Input | N | Syntax s | Current cop s | Preserved old cop s |
|---|---:|---:|---:|---:|
| Single attribute | 100 | 0.066 | 0.125 | 0.066 |
| Single attribute | 200 | 0.034 | 0.408 | 0.131 |
| Single attribute | 400 | 0.038 | 1.569 | 0.378 |
| Single attribute | 800 | 0.048 | 6.334 | — |
| Plural attributes | 100 | 0.034 | 0.085 | 0.065 |
| Plural attributes | 200 | 0.036 | 0.242 | 0.112 |
| Plural attributes | 400 | 0.040 | 0.881 | 0.294 |
| Plural attributes | 800 | 0.053 | 3.488 | — |
| Attribute block | 100 | 0.042 | 0.187 | 0.080 |
| Attribute block | 200 | 0.046 | 0.629 | 0.169 |
| Attribute block | 400 | 0.053 | 2.441 | 0.509 |
| Attribute block | 800 | 0.064 | 9.882 | — |

Old serializer comparisons are one repetition each; current values are medians of two. At 400 attributes current single/plural/block checks are ~4.15x/~2.99x/~4.80x slower than the preserved binary, so current shared traversal is a substantial part of this cost. The original algorithm still repeats owner scans and merits an index after the shared fix.

This also affects **compliant, zero-offense input**: with 800 public attributes and 800 differently named implementation methods, single attributes take 8.722s and blocks 12.050s, reporting zero offenses. Diagnostic emission is not the cause.

Recommendation: one per-source owner/parent method-name index, then O(1) lookup for each candidate; share where semantics allow. Preserve the deliberate difference: PreferAttribute rules require the nearest class/module/singleton owner; AvoidMethodAndBlock currently searches the direct parent subtree and can see nested methods. Do not accidentally merge those scopes or change symbol-only handling.

### 2. Rescue handler summaries and class-call ownership indexes

| Input | N nesting | Syntax s | Current cop s |
|---|---:|---:|---:|
| Rescue handlers, N offenses | 50 | 0.041 | 0.140 |
| Rescue handlers, N offenses | 100 | 0.042 | 0.784 |
| Rescue handlers, N offenses | 200 | 0.044 | 5.999 |
| Nonworker classes, zero offenses | 50 | 0.042 | 0.119 |
| Nonworker classes, zero offenses | 100 | 0.042 | 0.616 |
| Nonworker classes, zero offenses | 200 | 0.042 | 4.480 |
| Matching module prefixes, zero offenses | 50 | 0.041 | 0.055 |
| Matching module prefixes, zero offenses | 100 | 0.041 | 0.135 |
| Matching module prefixes, zero offenses | 200 | 0.042 | 0.741 |

`RescueMustReportError::reports_in` repeatedly scans nested handler subtrees. Summarize reporter presence once per subtree/postorder, preserving current treatment of nested reporters, bare raise vs local binding, safe navigation, exception class and handler boundaries. Do not tighten semantics merely because a narrower rule sounds preferable.

`ExplicitSidekiqQueue::class_calls` traverses every class subtree and then walks ancestors to find ownership, including nonworkers. Index class-owned calls once, or prune/index nested class/module ownership while retaining the current single-statement exclusion and worker/queue rules. At 200 nested nonworkers, current cost is 4.480s with zero findings. Much of the depth amplification is the shared iterator, so remeasure before implementing a second optimization.

`NoUndiscoverableAutoLoadedConstants::names::parent` still collects/reverses all ancestors and rebuilds namespace strings per definition. Per-source root/acronym caching was already implemented; it does not remove namespace reconstruction. Cache namespace states for owners/definitions or propagate them in one traversal, preserving absolute constants, singleton classes, Class.new/Module.new assignments, class_eval, unsupported dynamic receivers and acronym behavior. The 200-module test is ~0.741s, no offenses; lower priority than serializer/rescue/walker issues.

### 3. Redis transaction classification and expiration fast path

All transaction sets below are compliant: `redis.multi do |t|; ... t.set("a", "v", ex: 60) ...; end`, and all produce zero findings.

| Set calls | Syntax s | NoSetWithoutExpiration s |
|---:|---:|---:|
| 100 | 0.042 | 0.046 |
| 200 | 0.045 | 0.053 |
| 400 | 0.049 | 0.080 |
| 800 | 0.059 | 0.177 |

`redis_expiry` classifies the transaction before testing expiration. `transaction_body` materializes all body statements to check length >=2, and `transaction` lowercases the **whole transaction text** for every inner set. The former walks/allocates and latter rescans the same N-sized text N times.

Recommendation: check `has_expiration` before receiver/transaction classification (safe because a valid expiration never produces an offense regardless of receiver). Memoize transaction metadata/first parameter once per block; use a comment-filtered two-item iterator to test body size. Raw named_child_count is not equivalent because comments are filtered today. Preserve the existing whole-text Redis marker policy and >=2 statement quirk unless a separate parity change is intended. Existing source caches can host per-body metadata; this is not re-proposing the already implemented NoSetex lexical fix.

### 4. Lazy shared-model seeding and bare-identifier routing

A source containing only N ordinary local assignments has no `setex`, sleep, foreign-key or periodic-timer candidate. Three repeats per scope:

| Locals | Existing model s | Model + NoSetex s | Syntax s | Three bare-identifier custom rules s |
|---:|---:|---:|---:|---:|
| 1000 | 0.049 | 0.052 | 0.050 | 0.050 |
| 10000 | 0.125 | 0.131 | 0.114 | 0.120 |
| 50000 | 0.478 | 0.508 | 0.400 | 0.444 |

Existing model = Lint/UnusedMethodArgument. Adding gated NoSetex to that model, with no matching call, isolates eager LocalIdentifiers index seeding from bare-identifier routing: 50k locals add ~30ms (0.478 to 0.508s; user CPU 0.396 to 0.430s). Build or seed this index only when a candidate actually needs lexical membership, preserving the once-per-file parse/model reuse and exact byte semantics. The final design must avoid reverting to repeated parsing or copying an entire model per query. Scope is shared framework overhead; generic consumers can also trigger it, so it is not all attributable to custom28 on the real corpus.

Three identifier rules = NoSleep, NoForeignKeys, AnyCable/PeriodicalTimers, explicitly enabled for the same file. With no FileModel cop active and none of their names present, 50k locals add ~44ms over Syntax-only (0.400 to 0.444s). The method gates apply to calls, while these rules still receive every identifier. A complete bare-identifier name gate before checker callbacks can remove these irrelevant dispatches. Keep local-variable vs vcall semantics, parameter/name/assignment slots and per-cop enablement intact. This is measurable but lower impact than multi-second subtree costs.

## Method, validity and artifacts

- Release binary and config held fixed; cache reads disabled, filesystem warm, all timing runs sequential and no builds. A small Ruby compile validation ran during the tail of extra probes; no heavy concurrent scan/build ran.
- Representative sample is the previously recorded deterministic 632-file sample (2,711,452 bytes). Sample scopes repeated three times; one full sequential pair; synthetic current scopes generally two repeats; model controls three; preserved serializer comparisons one, rescues two. Do not rank small millisecond differences as precise function profiles.
- All 37 distinct generated source variants (including the intentionally aborted 400 nesting input) compile with pinned Ruby 3.4.9 without execution; Rust Syntax-only controls independently report no syntax offenses. All candidates expected clean in the tables report zero findings; primary serializer and rescue cases report exactly N.
- Main synthetic scopes use a temporary isolated config explicitly enabling custom cops with Include **/*; they do not claim the real Wistia config enables every cop on every path. No files in the application were rewritten.
- The parent/sibling algorithm diagnosis is supported by the local Cargo registry dependency source (tree-sitter 0.26.13 `src/node.c`), independently checked by the root reviewer. The timing regressions are measured, not inferred solely from static complexity.
- Raw timing rows: `/tmp/rr-custom-next-profile.json` (173 rows). Runner: `/tmp/rr-custom-next-profile.py`, resume/extra scripts beside it. The main runner was intentionally interrupted at rescue400; that aborted result is excluded from medians and recorded here as >30s only.
- Syntax validation: `/tmp/rr-custom-next-syntax.log` and source variants JSON. Same-source preserved/current complete-offense equality: `/tmp/rr-custom-next-regression.json`.

## Coverage inventory

All 28 were included in the custom group ablation and reviewed with the parallel static auditors. Targeted scaling probes focus on the confirmed costs above; this is not a claim of per-function sampled CPU profiles for every rule. perf counters are unavailable under this host's perf_event_paranoid=4; no bypass attempted.

- `AnyCable/InstanceVars`
- `AnyCable/PeriodicalTimers`
- `AnyCable/StreamFrom`
- `Cops/CreateTableMustIncludeCharset`
- `Cops/CreateTableMustIncludeCollation`
- `Cops/EnforceNoShowInPolicies`
- `Cops/EnforceScopeInPolicies`
- `Cops/ExplicitSidekiqQueue`
- `Cops/NoCSVGenerate`
- `Cops/NoCallbackSkippingProtectedWrites`
- `Cops/NoConstantDefinitionsInSpecs`
- `Cops/NoContentIndexCallsInBackfills`
- `Cops/NoDirectLanternConfigBuilderMutators`
- `Cops/NoForeignKeys`
- `Cops/NoRequireOnAutoLoadedPaths`
- `Cops/NoSetWithoutExpiration`
- `Cops/NoSetex`
- `Cops/NoSidekiqReceive`
- `Cops/NoSleep`
- `Cops/NoTopLevelSideEffectsInAutoloadedFiles`
- `Cops/NoUndiscoverableAutoLoadedConstants`
- `Cops/RescueMustReportError`
- `Custom/RequireRespondToAny`
- `Serializers/AvoidCapitalizedKeys`
- `Serializers/AvoidKeyAndBlock`
- `Serializers/AvoidMethodAndBlock`
- `Serializers/PreferAttributeBlockOverAttributeAndMethod`
- `Serializers/PreferAttributeBlockOverAttributesAndMethods`
