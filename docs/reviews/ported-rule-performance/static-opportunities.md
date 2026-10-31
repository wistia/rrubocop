# Static performance audit of all 98 retained ports

Reviewed HEAD `832a13f9a10842551c09eef37fe3dc513b81fbd5`, registry implementations and shared helpers. This was read-only; no builds, source edits, or tests were run. Companion inventory: `/tmp/rr-performance-static-inventory.json`. Its 98 names exactly equal the unique names in `tests/fixtures/ported-reference.json`: 28 app-specific and 70 generic, with 2,024 reference cases. Performance estimates below are algorithmic expectations, not measured time savings. The profiling agent owns measured benchmarks.

## Ranked recommendations

1. **Gate the 50 call-interested Rule registrations by method name.** `src/cop/ported/mod.rs:23` supplies only node-kind interest; `src/cop/walker.rs:111` already supports `interested_call_names`. Consequently each eligible call currently invokes all enabled call-interested Rule checkers, with repeated method extraction, safe-navigation probing, dispatch and empty-result handling. Add complete static method-name lists to registrations, retaining nongated non-call kinds. Expected broad low-risk reduction in dispatch work, especially generic rules enabled across the codebase; benefit is proportional to irrelevant call/check combinations, not offense count. Preserve the separate bare-identifier handling for sleep/add_foreign_key/periodically/define_method, receiver checks, operator methods, safe navigation and multi-name rules. Do not gate semantic wildcard rules with an incomplete list. Existing 2,024 reference cases should be rerun through the real walker, plus equivalence checks on bare identifiers and call/command syntax; direct checker-only tests cannot prove dispatch coverage.

2. **Reuse the per-file lexical model and pre-index identifier membership.** `context/bindings.rs:3` reparses the entire source, rebuilds FileModel, then scans all scopes/entries/reads/writes for every local_identifier query. This is file-sized work for one byte lookup. Ten ports can reach the helper, including normal diagnostic paths in NoSleep, NoForeignKeys, PeriodicalTimers, TopLevelMethodDefinition, SelfAssignment, SafeNavigationWithEmpty, RescueMustReportError and OutOfRangeRegexpRef; EvalWithLocation and NoSetex also use it to assess safe edits. Feed the existing file model into Context or lazily create it once per file; compute exactly the same set of intro/read/write byte positions once and query a HashSet or sorted vector. Expected potentially large benefit on files with many such candidates, but rare-name guards already exclude most nodes. Do not infer hotness from the dramatic worst case alone: NoSetex's argument-stability path requires an actual setex call. Preserve current membership semantics, parser scope boundaries and underscore/binding/super behavior; an approximate name-only symbol table could change diagnoses or make edits unsafe. Existing scope/regression fixtures cover several tricky cases; add multiple queries in one file and parity checks for reads, writes, block shadows and parameter positions.

3. **Resolve autoload context once per file; scope filesystem caches to a lint operation.** NoUndiscoverableAutoLoadedConstants (`difficult/autoload.rs:8`) repeatedly discovers the project via ancestor filesystem stats (`location.rs:42`), retrieves a global Mutex-protected Arc of acronyms, recomputes expected file name and splits both names for every definition. Read-only constant references are already cheaply excluded. Compute root/relative path/expected name/acronym handle once per source; pre-normalize acronym case to avoid repeated to_lowercase allocation in camel_word. NoRequireOnAutoLoadedPaths (`wistia_runtime/autoload.rs:5`) repeatedly discovers .rubocop.yml and enumerates app and not_production directories for each literal require; memoize the root directory list and (root, required) existence within one lint operation. Expected useful on files with many definitions/requires; no claimed corpus-wide bottleneck, and these cops have path/config filtering. Preserve Gemfile-first root semantics for constant discovery, nested .rubocop.yml regression, concern/STI handling, exact acronym precedence and filename case. The two cops currently use different root policies: caching must preserve them unless separately fixing parity. Avoid adding process-global permanent negative filesystem caches, especially in MCP/watch use; existing acronym cache itself can become stale after inflections change. Test changes between lint operations, concurrency, missing markers and multiple project roots.

4. **Replace materialized descendant/ancestor collections with short-circuit walkers, then remove repeat scans.** `context.rs:103` recursively constructs a Vec at every visited subtree, extends child vectors into parent vectors, and callers often immediately use any/find. Streaming preorder iteration or an explicit predicate visitor avoids whole-subtree work after the first match and reduces allocation; retain exact preorder, inclusion of the root and comments where relevant. DuplicateRequire (`lint_duplicates.rs:74`) scans/materializes its entire Ruby parent for each require despite take_while; track seen canonical requirements per Ruby parent or at least stream only the prefix. This is quadratic in same-parent requires. The five canonical consumers recursively allocate formatted strings and intermediate Vec/join (`context.rs:149`); first write the existing canonical encoding into a single buffer, which preserves exact equivalence. Hash-only canonicalization needs collision handling; AST equality is not automatically the same semantic equivalence. Expected broad allocation reduction and improved large-file behavior, with uncertain wall-time effect. Preserve comment-sensitive EmptyWhen, Ruby-parent boundaries, heredoc ordering and scope ownership. All reference cases plus focused same-parent/different-parent duplicate cases and interpolation/comments should gate this work.

5. **Share regexp analysis, or at least make capture-only scans avoid token emission.** Pattern::sets, escapes and captures each build/scans independently; scanner::captures currently calls the full run and pushes unused escape/class Token records (`style_regexp/scanner.rs:24`). RedundantRegexpCharacterClass, RedundantRegexpEscape, MixedRegexpCaptureTypes and sometimes OutOfRangeRegexpRef inspect the same regex. A per-node summary can amortize scans; a capture-only output mode is a smaller first step. OutOfRange already has a cheap `$[1-9]` source prefilter: keep it, and distinguish files triggering its full custom walk from ordinary files. Expected local benefit on regex-heavy inputs, likely lower priority for average Ruby files. Scanner state (inline/scoped x options, classes, escape sequences, interpolation and named captures) is shared correctness-sensitive logic: suppress output only, not scanning state transitions. Existing scanner tests and pinned reference fixtures are useful; rerun regex parity and real pipeline cases before accepting a rewrite.

6. **Index ShadowedArgument provenance per file/scope, preserving Ruby reference semantics.** Shared FileModel is already reused and arguments without writes already return early. For eligible written arguments, `shadow/references/implicit.rs:18` enumerates an owner subtree again per argument, finds scope by scanning every scope plus tree lookup, and checks vcall_sites with linear contains. Build a scope-node/range index and cache implicit binding/super sites per owning scope once; use byte membership lookup for vcall sites. references.rs also scans same-name entries across scopes for assignment references. Expected benefit mainly on argument-heavy methods with overwrites or binding/super, not all methods. Risk is medium: nested block/shadow scope and Ruby VariableForce-compatible branch semantics are deliberate. Do not limit to the current entry as a shortcut or delete implicit references. Gate on nested scopes, read-before-write, mutually exclusive branches, super, binding and captured underscore arguments; compare full diagnostic ranges and counts.

## Additional lower-priority work

* **Separate correction feasibility from rendering (52 correctable registrations).** Rule constructs Findings and their edit Strings before emit discovers corrections=None; autocorrect-disabled config clears edits after building them. UselessTimes can compile parameter regexes and walk/rewrite bodies even in normal lint; NoSetex proves argument stability even without requested correction. This is not a safe one-line skip: Diagnostic.correctable is currently inferred from Finding.edit presence, so skipping feasibility would change JSON/UI behavior. Introduce an explicit correction capability/feasibility bit and render edits lazily, preserving diagnostics and safe/unsafe correction gating. Expected benefit is proportional to actual correctable offenses; existing Wistia corpus has very few among the newly retained set, so this is likely lower than general dispatch work. Validate off/-a/-A/config-disabled output and corrected flags with the pinned safe-refusal cases.
* **Compile configuration regexes once.** Style/ClassEqualityComparison compiles every AllowedPatterns expression for each candidate comparison (`style_conditions/class_equality.rs:62`) and collects AllowedMethods repeatedly. Prepare valid/invalid regex results per resolved cop config. Current empty AllowedPatterns config is already cheap, so this is conditional rather than a demonstrated Wistia hotspot. Preserve regex syntax and invalid-pattern behavior; do not substitute a different regex engine.
* **Small allocation cleanups** such as repeated args/kids Vec creation, Finding message cloning in emit and Vec result for zero/one findings are widespread but individually bounded. Benchmark after higher-level changes; changing every checker API first creates review cost without an established bottleneck.

## Coverage classification

Every registered retained port is listed below and individually tagged in the JSON. Untagged narrow handlers were reviewed as bounded local/node or class-subtree work; no rule is recommended for removal to improve speed. Tags are candidates, not proof that a rule is hot on this corpus. Model and filesystem costs are input/config-dependent. All retained rules remain valuable behavior; performance work should preserve the same checks.

### difficult (5)

`Cops/NoUndiscoverableAutoLoadedConstants`, `Lint/MixedRegexpCaptureTypes`, `Lint/OutOfRangeRegexpRef`, `Lint/ShadowedArgument`, `Style/EvalWithLocation`

### lint_calls (8)

`Lint/ErbNewArguments`, `Lint/IdentityComparison`, `Lint/RaiseException`, `Lint/RedundantWithIndex`, `Lint/RedundantWithObject`, `Lint/TrailingCommaInAttributeDeclaration`, `Lint/UriEscapeUnescape`, `Lint/UriRegexp`

### lint_duplicates (5)

`Lint/DuplicateCaseCondition`, `Lint/DuplicateElsifCondition`, `Lint/DuplicateRequire`, `Lint/DuplicateRescueException`, `Lint/EmptyWhen`

### lint_flow (10)

`Lint/FloatComparison`, `Lint/MultipleComparison`, `Lint/OrAssignmentToConstant`, `Lint/RegexpAsCondition`, `Lint/RescueType`, `Lint/ReturnInVoidContext`, `Lint/SafeNavigationConsistency`, `Lint/SafeNavigationWithEmpty`, `Lint/SelfAssignment`, `Lint/TopLevelReturnWithArgument`

### lint_literals (7)

`Lint/AmbiguousAssignment`, `Lint/BooleanSymbol`, `Lint/DeprecatedConstants`, `Lint/EmptyExpression`, `Lint/InterpolationCheck`, `Lint/NestedPercentLiteral`, `Lint/TripleQuotes`

### retained (14)

`Cops/NoSetex`, `Lint/NumberedParameterAssignment`, `Lint/RefinementImportMethods`, `Lint/UselessTimes`, `Migration/DepartmentName`, `Serializers/PreferAttributeBlockOverAttributeAndMethod`, `Serializers/PreferAttributeBlockOverAttributesAndMethods`, `Style/ColonMethodDefinition`, `Style/EmptyBlockParameter`, `Style/KeywordParametersOrder`, `Style/MixinGrouping`, `Style/MultilineWhenThen`, `Style/TrailingBodyOnMethodDefinition`, `Style/TrailingMethodEndStatement`

### style_blocks (6)

`Style/FileRead`, `Style/FileWrite`, `Style/HashConversion`, `Style/HashExcept`, `Style/MapCompactWithConditionalBlock`, `Style/TrailingCommaInBlockArgs`

### style_calls (8)

`Style/Dir`, `Style/RandomWithOffset`, `Style/RedundantFileExtensionInRequire`, `Style/RedundantSort`, `Style/SlicingWithRange`, `Style/StderrPuts`, `Style/StringChars`, `Style/UnpackFirst`

### style_conditions (8)

`Security/CompoundHash`, `Style/ClassEqualityComparison`, `Style/MixinUsage`, `Style/OrAssignment`, `Style/RedundantAssignment`, `Style/RedundantConditional`, `Style/TopLevelMethodDefinition`, `Style/UnlessLogicalOperators`

### style_regexp (2)

`Style/RedundantRegexpCharacterClass`, `Style/RedundantRegexpEscape`

### wistia_calls (10)

`AnyCable/PeriodicalTimers`, `Cops/CreateTableMustIncludeCharset`, `Cops/CreateTableMustIncludeCollation`, `Cops/NoCSVGenerate`, `Cops/NoCallbackSkippingProtectedWrites`, `Cops/NoContentIndexCallsInBackfills`, `Cops/NoDirectLanternConfigBuilderMutators`, `Cops/NoForeignKeys`, `Cops/NoSidekiqReceive`, `Cops/NoSleep`

### wistia_runtime (9)

`AnyCable/InstanceVars`, `AnyCable/StreamFrom`, `Cops/NoRequireOnAutoLoadedPaths`, `Cops/NoSetWithoutExpiration`, `Cops/RescueMustReportError`, `Custom/RequireRespondToAny`, `Serializers/AvoidCapitalizedKeys`, `Serializers/AvoidKeyAndBlock`, `Serializers/AvoidMethodAndBlock`

### wistia_scopes (6)

`Cops/EnforceNoShowInPolicies`, `Cops/EnforceScopeInPolicies`, `Cops/ExplicitSidekiqQueue`, `Cops/NoConstantDefinitionsInSpecs`, `Cops/NoTopLevelSideEffectsInAutoloadedFiles`, `Rails/ReversibleMigrationMethodDefinition`

## Measurement constraints

Individual-cop CLI timing includes process startup, file discovery, config resolution and parsing, and path Includes can change the inspected file count. Subtracting unmatched baselines is misleading. Report dynamic results separately with equal file populations and enabled model-building controls. The profiling agent reports no dramatic individual outlier on its initial 140-file set; that supports prioritizing shared dispatch/model/allocation machinery, but does not prove absence of worst-case inputs. Static audit makes no measured speedup promises.
