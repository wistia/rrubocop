# Performance review of all 98 added ports

**Follow-up:** The concrete optimization categories are implemented in the [parity fixes and optimization record](port-fixes-and-optimizations.md), including controlled before/after results. The remainder preserves the original audit.

Read-only audit of `832a13f9a10842551c09eef37fe3dc513b81fbd5` in `the rrubocop checkout`. No source edits or builds. The working tree remained clean. Coverage: **98 ports = 28 app-specific + 70 generic**, 2,024 pinned reference cases; 672 registered cops overall.

## Findings

On the same release binary, current Wistia config, and 14,762-file corpus, uncached full lint took **31.53s**; excluding the 98 additions took **26.10s**. The additions therefore cost **5.43s (20.8%)** in this sequential pair, with **42.33s aggregate user CPU** added. This is a controlled ablation, unlike comparisons of historical 35/40/50-second runs. One full pair is directional evidence, not a confidence interval or promised CI runtime.

Repeated fixed-sample runs independently show an aggregate cost, without identifying one dominant rule. Shared dispatch/configuration/allocations are the broad opportunity. One candidate-dependent worst case is confirmed: NoSetex repeatedly reparses the whole file to resolve a local identifier, causing quadratic growth even when autocorrection is off.

## Measurements

| Scope, real Wistia config; 632 fixed files | Median wall s | Median user CPU s | Repeats |
|---|---:|---:|---:|
| syntax | 0.960 | 1.361 | 3 |
| existing574 | 2.266 | 8.814 | 3 |
| all672 | 2.583 | 10.535 | 3 |
| added98 | 1.228 | 3.412 | 3 |
| custom28 | 1.006 | 1.732 | 3 |
| general70 | 1.201 | 3.159 | 3 |
| last5_correct | 1.010 | 1.803 | 3 |
| autoload_correct | 0.929 | 1.022 | 3 |

The sample is a deterministic hash-ranked 4% sample within directory/feature buckets, with at least 3 files where available: 632 files / 2,711,452 bytes. Specs contribute 234 files; app/graphql84, app/lib47, app/models43, db52, etc. Manifest and all bucket counts are in the JSON. Each group ran sequentially in three shuffled rounds. All98 also received one individual run on a fixed 140-file subset; the complete inventory below records each run in JSON. These individual runs are startup/config dominated (roughly 0.69–0.77s), have different cop path eligibility, and are deliberately **not ranked as precise rule costs**. No large individual outlier appeared on that subset.

Only-group numbers retain real cop Includes/Excludes/enabled settings: autoload-only can do less parsing than Syntax-only because files with no active cop return before parsing. They cannot be subtracted indiscriminately or summed. Most ports perform bounded local checks and are inexpensive individually; repeated aggregate dispatch and shared helpers are more promising than removing checks.

### Separate parse/engine baseline from shared model activation

The engine parses once per active file, runs syntax checks, builds FileModel once if any model cop requires it, and builds CodeMap once if a plain-source cop requires it. ShadowedArgument reuses this model; it does not introduce another ordinary full parse. To control enabled-state differences, a separate fixed-sample experiment used `--force-default-config` and the exact same file list:

| Controlled default-config scope | Median wall s | Median user CPU s |
|---|---:|---:|
| defaults_syntax | 0.294 | 0.813 |
| defaults_existing_model | 0.327 | 1.035 |
| defaults_shadow | 0.314 | 0.948 |
| defaults_model_plus_shadow | 0.331 | 1.045 |
| defaults_plain_source | 0.308 | 0.898 |

`defaults_existing_model` = Lint/UnusedMethodArgument; plus_shadow adds Lint/ShadowedArgument; plain_source = Lint/OutOfRangeRegexpRef. Each control has 3 nonoverlapping sequential repeats. Existing model plus shadow is essentially tied with existing model alone at this resolution, while standalone shadow includes activating the shared model. OutOfRange also activates CodeMap before its cheap `$[1-9]` prefilter, so its standalone overhead includes shared framework work. This separates activation from marginal checker cost, but does not pretend to isolate exact parse/model/function percentages. The normal full-suite baseline already has model/source-phase users.

### Confirmed worst case, diagnostic mode

A standalone temporary project enables Cops/NoSetex and contains a single method with N `redis.setex("a", ttl, value)` calls and local method parameters. No autocorrection flags were passed; all N offenses were found. Two sequential measurements per input:

| N calls | Syntax-only wall s | NoSetex wall s |
|---:|---:|---:|
| 50 | 0.031 | 0.117 |
| 100 | 0.031 | 0.372 |
| 200 | 0.033 | 1.377 |
| 400 | 0.038 | 5.407 |

Doubling N approximately quadruples NoSetex cost; 400 calls take ~5.4 seconds for a ~12KB file while syntax remains ~0.04s. `context/bindings.rs:3::local_identifier` reparses and rebuilds FileModel on every query; NoSetex calls it for both reordered identifier arguments to decide edit safety. This is O(K*N) work for K candidate identifiers in an N-sized file, plus scans across all model entries. The synthetic result establishes the pathological behavior, not its frequency or a 5-second corpus-wide saving.

## Optimization priorities

1. **Share a per-file local-identifier index.** Feed/reuse the existing FileModel, or build it lazily once and index intro/read/write byte positions. Ten ports can reach this helper. This removes the measured quadratic behavior and repeated parse/model allocation. Preserve exact lexical scope and binding semantics; name-only guesses would weaken correction safety.
2. **Add complete method-name gates for 50 call-interested registrations.** Rule currently supplies node kinds but no `interested_call_names`, although the batched walker supports it. The generic70 account for most added sample CPU. Gating avoids irrelevant callbacks/method extraction for ordinary calls. The corpus-wide speedup is unmeasured; preserve bare identifiers, multi-name/operator rules and safe navigation. Also examine per-file enabled/config selection, which clones configs and scans filters independently of rule logic.
3. **Cache autoload context once per source/lint operation.** NoUndiscoverableAutoLoadedConstants repeats project-root filesystem probes, acronym-cache locking, expected-name generation and namespace splitting per definition. NoRequireOnAutoLoadedPaths re-enumerates app/not_production roots and stats paths per literal require. Preserve their existing distinct root policies, nested config semantics and concern/STI handling; avoid permanent negative caches in watch/MCP mode. The sampled autoload group was small, so prioritize after the broader shared work unless more representative require-heavy files prove otherwise.
4. **Replace allocating descendant/ancestor vectors with short-circuit iterators and indexes.** `context.rs:103` recursively allocates and copies subtree vectors, worst-case O(N*depth); canonical strings are rebuilt recursively. DuplicateRequire, serializers, heredoc matching and duplicate-condition checks repeat scans. Preserve traversal order/comments/byte locations. This is an algorithmic opportunity without a measured corpus savings figure.
5. **Split correction feasibility from edit rendering.** 52 ports are marked correctable; checkers eagerly build edits even when corrections=None or AutoCorrect:disabled, because correctability is inferred from an edit. Avoid expensive rendering after retaining a cheap accurate correctable flag. NoSetex proves safety classification itself may remain semantic and expensive without sharing a model. Do not simply skip edit construction and change JSON correctability.
6. **Share regex analysis / cache configurable regex compilation where profitable.** Four regex-related additions rescan captures/escapes/classes independently; class-equality allowed-pattern checks compile Regex repeatedly. Avoid a second OutOfRange tree walk when no relevant references, but note its current `$[1-9]` prefilter already avoids most work. Small corpus share suggests lower priority until profiled on regex-heavy input.

All benefits above are expectations except the measured NoSetex growth and aggregate ablations. No optimized implementation was built, so **no speedup claim is made for a proposed fix**.

## Reproduction and limitations

Host: 8 effective CPUs; cpu.stat nr_throttled=0/nr_periods=0, no exposed cpu.max. `perf stat` was denied by perf_event_paranoid=4; no sysctl changes or intrusive attaches were attempted. User/system CPU comes from RUSAGE_CHILDREN and sums parallel execution. Filesystem caches were warm and were not dropped. Result cache reads are disabled; the CLI still writes result-cache entries. Other agents performed light read-only reviews; no shared builds ran.

Artifacts: `/tmp/rr-perf-run.py`, `/tmp/rr-perf-followup.py`, `/tmp/rr-perf-model.py`, `/tmp/rr-perf-synthetic.py`, `/tmp/rr-perf-sample.txt`; raw timings and inventory in `/tmp/rr-performance-all-ports.json`. The report omits exploratory malformed-name groups, inactive model controls, and the first model control round that overlapped a separate process.

Full pair reproduction from `/home/ubuntu/code/wistia-rrubocop-benchmark`:

```sh
RR=the rrubocop checkout/target/release/rrubocop
ADDED=$(python3 -c 'import json; print(",".join(json.load(open("/tmp/rr-performance-inputs.json"))["all_added"]))')
/usr/bin/time -v "$RR" --cache false --format json --except "$ADDED" > /tmp/rr-excluding98.json
/usr/bin/time -v "$RR" --cache false --format json > /tmp/rr-all672.json
```

Avoid timing 98 independent full corpus runs: shared startup/parse/model costs overlap. To benchmark a future optimization, repeat the same binary/config/file-list comparisons and retain correctness/autocorrect equivalence checks.

## Complete 98-port coverage

Every row has a fixed140-file single-cop timing in the JSON plus static checker/source/kind/correction inventory from the companion static audit. Untagged narrow handlers are bounded local checks; none is recommended for retirement for speed.

**difficult (5)**: `Cops/NoUndiscoverableAutoLoadedConstants`, `Lint/MixedRegexpCaptureTypes`, `Lint/OutOfRangeRegexpRef`, `Lint/ShadowedArgument`, `Style/EvalWithLocation`.

**lint_calls (8)**: `Lint/ErbNewArguments`, `Lint/IdentityComparison`, `Lint/RaiseException`, `Lint/RedundantWithIndex`, `Lint/RedundantWithObject`, `Lint/TrailingCommaInAttributeDeclaration`, `Lint/UriEscapeUnescape`, `Lint/UriRegexp`.

**lint_duplicates (5)**: `Lint/DuplicateCaseCondition`, `Lint/DuplicateElsifCondition`, `Lint/DuplicateRequire`, `Lint/DuplicateRescueException`, `Lint/EmptyWhen`.

**lint_flow (10)**: `Lint/FloatComparison`, `Lint/MultipleComparison`, `Lint/OrAssignmentToConstant`, `Lint/RegexpAsCondition`, `Lint/RescueType`, `Lint/ReturnInVoidContext`, `Lint/SafeNavigationConsistency`, `Lint/SafeNavigationWithEmpty`, `Lint/SelfAssignment`, `Lint/TopLevelReturnWithArgument`.

**lint_literals (7)**: `Lint/AmbiguousAssignment`, `Lint/BooleanSymbol`, `Lint/DeprecatedConstants`, `Lint/EmptyExpression`, `Lint/InterpolationCheck`, `Lint/NestedPercentLiteral`, `Lint/TripleQuotes`.

**retained (14)**: `Cops/NoSetex`, `Lint/NumberedParameterAssignment`, `Lint/RefinementImportMethods`, `Lint/UselessTimes`, `Migration/DepartmentName`, `Serializers/PreferAttributeBlockOverAttributeAndMethod`, `Serializers/PreferAttributeBlockOverAttributesAndMethods`, `Style/ColonMethodDefinition`, `Style/EmptyBlockParameter`, `Style/KeywordParametersOrder`, `Style/MixinGrouping`, `Style/MultilineWhenThen`, `Style/TrailingBodyOnMethodDefinition`, `Style/TrailingMethodEndStatement`.

**style_blocks (6)**: `Style/FileRead`, `Style/FileWrite`, `Style/HashConversion`, `Style/HashExcept`, `Style/MapCompactWithConditionalBlock`, `Style/TrailingCommaInBlockArgs`.

**style_calls (8)**: `Style/Dir`, `Style/RandomWithOffset`, `Style/RedundantFileExtensionInRequire`, `Style/RedundantSort`, `Style/SlicingWithRange`, `Style/StderrPuts`, `Style/StringChars`, `Style/UnpackFirst`.

**style_conditions (8)**: `Security/CompoundHash`, `Style/ClassEqualityComparison`, `Style/MixinUsage`, `Style/OrAssignment`, `Style/RedundantAssignment`, `Style/RedundantConditional`, `Style/TopLevelMethodDefinition`, `Style/UnlessLogicalOperators`.

**style_regexp (2)**: `Style/RedundantRegexpCharacterClass`, `Style/RedundantRegexpEscape`.

**wistia_calls (10)**: `AnyCable/PeriodicalTimers`, `Cops/CreateTableMustIncludeCharset`, `Cops/CreateTableMustIncludeCollation`, `Cops/NoCSVGenerate`, `Cops/NoCallbackSkippingProtectedWrites`, `Cops/NoContentIndexCallsInBackfills`, `Cops/NoDirectLanternConfigBuilderMutators`, `Cops/NoForeignKeys`, `Cops/NoSidekiqReceive`, `Cops/NoSleep`.

**wistia_runtime (9)**: `AnyCable/InstanceVars`, `AnyCable/StreamFrom`, `Cops/NoRequireOnAutoLoadedPaths`, `Cops/NoSetWithoutExpiration`, `Cops/RescueMustReportError`, `Custom/RequireRespondToAny`, `Serializers/AvoidCapitalizedKeys`, `Serializers/AvoidKeyAndBlock`, `Serializers/AvoidMethodAndBlock`.

**wistia_scopes (6)**: `Cops/EnforceNoShowInPolicies`, `Cops/EnforceScopeInPolicies`, `Cops/ExplicitSidekiqQueue`, `Cops/NoConstantDefinitionsInSpecs`, `Cops/NoTopLevelSideEffectsInAutoloadedFiles`, `Rails/ReversibleMigrationMethodDefinition`.


## Supporting static audit

See the [ranked static analysis](ported-rule-performance/static-opportunities.md) and [complete 98-rule inventory](ported-rule-performance/inventory.json). Raw timing records remain in `/tmp/rr-performance-all-ports.json` on the review machine. Proposed optimizations are not implemented.
