# RSpec-family efficiency review

Snapshot: `2db3f585334e669e5b1466ded9ffadb1e7dff0ec`.
Read-only: no implementation changes, builds, original-Ruby matrices, or full-repository benchmarks.

## Coverage and evidence

- **106/106 implemented registrations screened:** 104 RSpec, two RSpecRails. No implemented/registered Minitest rules or additional RSpec-factory namespace rules were found. Actual FactoryBot rules belong to the framework reviewer.
- All 106 runtime callback/helper-use sites were statically screened (11,454 runtime lines in rule files), with detailed inspection of the high-risk paths and seven RSpec-specific helper modules.
- The coverage ledger is `/tmp/rrubocop-perf-rspec-inventory.json`; the full 106-row table is `/tmp/rrubocop-perf-rspec-coverage.md`. It records source paths, callback lines, routing, source hashes and finding references. Wistia baseline enablement: 96 yes, eight no, two absent from that Ruby inventory. All 106 were reviewed regardless of enablement.
- Associated helpers inspected: `helpers.rs`, `parity_helpers.rs`, `description.rs`, `separation.rs`, `subject_scopes.rs`, `ordering.rs`, `ruby_arguments.rs`. Common `ported/context/bindings.rs` and `shared/ruby_expression.rs` were inspected at the consumer boundary; engine/model redesign belongs to the core reviewer.
- Except F1's small synthetic check, the findings below are **source-based hypotheses**, not measured production costs. Priority is a recommended investigation order, not a claimed wall-clock ranking.

## Prioritized findings

### F1 — dynamic description decoding copies and reparses the whole file

**Highest demonstrated scaling concern; active Wistia rules.**

Locations:
- `src/cop/rspec/description.rs:26`: `interpolations` climbs to the program root and walks the entire tree to find interpolation nodes within one literal range.
- `description.rs:52`, `:64`, `:75`: each dynamic description can clone the entire source, splice markers, parse the entire surrogate source, and walk it again to find the original literal.
- `description.rs:184`, `:201`, `:215`: correction validation parses a literal fragment, but also repeats the old whole-file interpolation query twice.
- `src/cop/rspec/example_wording.rs:147`, `:155`, `:179`, `:189`: decode and correction preparation are reached per eligible description; rewriting and `safe_replacement` still run in ordinary lint mode even when there is no corrections sink.
- `src/cop/rspec/excessive_docstring_spacing.rs:88`, `:105`: the same shared decoder and correction safety work can be repeated by a second enabled cop.

Ordinary cost is approximately O(dynamic descriptions × file size/tree size), plus repeated literal validation on offending descriptions. Marker collisions can trigger retries; the loop bound is the source byte count. This retry bound is a pathological input concern, not an observed production scenario.

**Small measured check:** source-equivalent optimized binary 362240f (no Rust source diff to audited snapshot), only ExampleWording, virtual stdin, no correction, two repetitions per point. Static 64/256-example inputs took median 3.291/3.306 seconds, while dynamic 64/256 inputs took 3.424/5.427 seconds. The dynamic excess over the paired static control grew about 0.133→2.121 seconds for 4× descriptions/source size, consistent with the source-based quadratic mechanism. The CLI baseline/startup dominates these small timings; they are not production cop timings or an allocation profile. Raw inputs/results: `/tmp/rrubocop-perf-rspec-probes.json`.

A textual workload screen found 520 `it` lines containing `#{` across 270 of 5,869 spec Ruby files, maximum 13 such lines in one file. This is a textual candidate count, includes possible single-quoted false candidates and misses multiline/heredoc cases. It prevents treating the synthetic 256-descriptions case as representative Wistia frequency.

Opportunities: memoize decoded description text and interpolation ranges per immutable source/literal; share between the two cops; obtain normal interpolation children locally and reuse the existing delayed-heredoc span machinery; delay replacement rendering and safety validation until a correction is requested where diagnostic correctability is already known. Preserve negative cache results too.

Parity risks: interpolation bytes, physical line boundaries, `__LINE__`, heredoc placement, chained/dynamic string AST shapes, escaping and marker collisions. Do not replace these safeguards with plain string substitution or remove them in correction modes. Cache owned ranges/facts rather than borrowed nodes across different parse trees.

### F2 — ReturnFromStub scans receiver/argument subtrees before cheap eligibility

**High-confidence low-complexity improvement candidate; active.**

Locations: `src/cop/rspec/return_from_stub.rs:12–34`, `:60–66`, `:109–117`.

`contains_stub` runs on every interested call before checking the configured style, `and_return` selector for block style, or the existence of a block for and_return style. It descends receiver/arguments at its first step and all descendants beneath those; it does **not** automatically traverse the root call's own block body, so this is not a claim of a whole-file scan on every call. Deep argument/call chains can nevertheless be revisited from many ancestors.

Move the necessary cheap style-specific conditions ahead of this search: block style requires `and_return`; default style requires an attached ordinary block. If still material, cache a subtree's stub-presence fact. Rendering replacement strings after reporting also runs without a corrections sink and can be deferred.

Parity risks: the original accepts nested/wrapped receiver or argument chains, not just a direct receive call; retain this search when eligible. Numbered-block, safe-send and logical argument/hash guards must remain unchanged.

### F3 — repeated use/subject searches inside otherwise eligible groups

**Potentially quadratic for large groups; source-only.**

- `src/cop/rspec/let_setup.rs:48–81`, `:111–139`: after collecting helpers, every eligible `let!` starts a fresh traversal of the entire containing group looking for use. Approximate worst case O(eager helpers × group nodes). Build a per-group occurrence/name index once, keeping current receiver, omitted-keyword-value and lexical-local rules.
- `src/cop/rspec/instance_spy.rs:41–46`, `:47–76`, `:76–96`: collects all example descendants, then rescans that vector for each relevant assignment. Build the eligible have_received target-name facts once. Current code compares name bytes plus local-role checks, so silently changing it to binding-ID equality is a semantic change, not a performance refactor.
- `src/cop/rspec/named_subject/mod.rs:141`, `:172–185`, `:200–210`, `:222–246`: named_only searches ancestor bodies for every subject reference; duplicate reporting searches the shared diagnostics vector linearly. A per-scope subject fact and a per-cop emitted-position set can reduce rescans. Maintain nearest-definition and first-encounter behavior.
- `src/cop/rspec/implicit_block_expectation.rs:63–93`: each implicit expectation walks ancestor groups and searches their body for the applicable subject. This is another consumer of the same rich scope facts, with its own selection policy.

These rules differ in boundary and inheritance rules. Share an index of facts, not a single flattened policy or universal definition winner.

### F4 — duplicate grouping uses pairwise deep AST comparisons

**High potential on wide groups; higher parity risk.**

Locations:
- `src/cop/rspec/repeated_example.rs:45–63`.
- `src/cop/rspec/repeated_description.rs:34–47`.
- `src/cop/rspec/repeated_example_group_body.rs:69–85`.
- `src/cop/rspec/repeated_example_group_description.rs:50–80`.
- `src/cop/rspec/repeated_include_example.rs:58–68`.
- `src/cop/rspec/scattered_setup.rs:57–63`, `:76–85`, `:328–343`: metadata normalization/group membership adds nested pair searches.
- `src/cop/rspec/receive_messages.rs:249–280`: linear search through previously built groups, with repeated argument projections and expression comparison.

The repeated rules compare many pairs and repeatedly allocate argument vectors/decode literals; worst-case unique or late-matching groups can be O(G² × comparison work). Cache each argument projection and use collision-checked canonical buckets or conservative prefilters before deep comparison. Hooks can normalize their metadata once rather than rebuilding equality search structures for each comparison.

Required caveats: duplicate line-list messages can themselves have O(G²) total output when all groups duplicate; the optimization cannot remove that required output. Preserve stable source ordering. `shared/ruby_expression.rs:417–435` deliberately distinguishes `same_expression` from strict `eql_expression`. A weak text/signature hash is not interchangeable: numeric families, strict complex equality, symbol/dynamic-string shape, keyword hash grouping, regex flags/escapes and original AST normalization matter. Hash collisions must fall back to the existing comparator; equal expressions must never fall into different buckets. Do not reproduce stale upstream hash bugs or discard explicit existing safeguards.

### F5 — common group topology and ancestor facts are recomputed across cops

**Broad shared-model opportunity; source-only.**

- `src/cop/rspec/helpers.rs:151–160`, `:168–205`; `parity_helpers.rs:218–226`: ancestry and top-level-group discovery repeatedly climb to the root and rebuild the group list. InstanceVariable calls this for each non-assignment instance variable (`instance_variable.rs:101–104`); DescribeClass/DescribeMethod query it for candidate groups; program-level subject/path cops also repeat the same discovery.
- `src/cop/rspec/multiple_memoized_helpers.rs:69–115`, `:134–143`: each group collects own names and then revisits every ancestor group's eligible body. Deep groups can cause repeated ancestor-body work. **This rule is disabled in the pinned Wistia inventory**, so prioritize only for all-implemented coverage or another workload.
- `src/cop/rspec/described_class.rs:202–207`: independently traverses every described group's descendants, then performs several ancestry checks for matching constants (`:147–161`). Nested describe scopes repeat descendant work.
- `src/cop/rspec/subject_scopes.rs:10–54`, `subject_stub.rs:117–121`, `repeated_subject_call.rs:95–98`: subject definitions are collected separately, including two passes in SubjectStub, and ancestor-group lookup is repeated for each definition. Subject-name vectors are cloned during recursive traversal.
- `parity_helpers.rs:479–516`: scoped example collection is independently repeated by both repeated-example cops.

A source-local topology model can record group/example boundaries, ordinary versus numbered block roles, method boundaries, direct statement lists, subject/let definitions and ordered occurrences once. Keep separate policy-specific views for top-level wrapper unwrapping, included/shared groups, inherited names, methods, rescue/ensure and skipped metadata.

Already good: lexical local/binding and numbered-block data is cached via `ported/context/bindings.rs:5–18`; do not claim or introduce a replacement whole-file scan there. SourceFile's typed cache requires owned data; node IDs from a separately reparsed tree are not a portable identity for caller nodes.

### F6 — sibling, comment-line and ordering queries repeat linear scans

**Often lower-risk indexing/prefix work.**

`src/cop/rspec/separation.rs:8–29` rebuilds the parent code-sibling vector and searches it for each target; five EmptyLineAfter* cops reuse this helper. `:85–103` linearly searches the entire cached comment vector for each successive adjacent comment line. The comment collection itself is **already cached once** at `:41–58`; the opportunity is a line/range lookup index, not caching that traversal again. The heredoc descendant walk `:72–81` can reuse an existing per-subtree delayed-literal-end fact.

`src/cop/rspec/ordering.rs:276–285` recomputes a prefix helper count per ScatteredLet candidate. `:316–335` repeatedly scans crossed sibling ranges for all ordering styles. `leading_subject.rs:37–60` repeats sibling predicates, while EmptyLineAfterFinalLet traverses child lists to find the final helper before using the sibling helper.

Use sibling ranks, prefix counts and policy-specific range summaries. Correction safety depends on crossing eager hooks/helpers, duplicate definition names, include_examples, comment/directive placement, heredocs, semicolons and ownership; those guards cannot simply be skipped in read-only mode if they decide correctability. ScatteredSetup and ReceiveMessages already delay full source rendering/parsing until a corrections sink exists.

### F7 — prepare configuration regexes once

**Small, low-risk candidate with an actual nonempty Wistia configuration.**

- `src/cop/rspec/parity_helpers.rs:244–254` compiles patterns afresh.
- `src/cop/rspec/no_expectation_example.rs:58`: called once per eligible example. Wistia's pinned AllowedPatterns is `["^expect_", "^assert_"]`, so this is a real repeated compile path rather than an empty-option hypothetical.
- `variable_name.rs:50`, `context_wording.rs:67`, `indexed_let.rs:61–65` have similar per-candidate/group construction. Their pinned patterns are empty and IndexedLet is disabled, so materiality is conditional.
- `spec_file_path_format.rs:163` compiles a derived expected-path regex per candidate group. Existing fixed SPACE/NON_WORD and inflection patterns use OnceLock correctly.

Cache successful **and invalid** compilation per immutable pattern list/options or prepare per resolved cop configuration; key by configuration, since nested scopes can override it. Preserve current regex dialect, case/multiline flags and invalid-pattern policy. An operation-wide immutable cache is more useful than recompiling once per file when all files share the same configuration.

### F8 — repeated key decoding and correction preparation allocations

**Lower priority until allocation profiling.**

- `src/cop/rspec/sort_metadata.rs:67–80`: sort_by_key recomputes decoded/lowercased String keys during comparisons. Prepare keys once or consider cached-key sorting with the same stable ordering. Metadata lists are normally small.
- `src/cop/rspec_rails/inferred_spec_type.rs:42–58`, `:162–164`: path inference and formatted prefix strings repeat for every group, although the path/config are fixed per file. Cache inference per path/config. HttpStatus is already selector-routed and inspects only local arguments (`http_status.rs:37–61`).
- VariableDefinition (`variable_definition.rs:49–72`) and VerifiedDoubleReference (`verified_double_reference.rs:76–82`) build replacement literals in lint mode. The parsing helper `parity_helpers.rs:445–463` is named literal_replacement and has no runtime call sites in this snapshot; it is excluded from the runtime parse opportunity. InstanceSpy prepares source slices/replacements before discovering that no sink will accept them. Delay work that is purely correction text; retain diagnostic/correctability decisions.
- `parity_helpers.rs:156–167` and `rspec/ruby_arguments.rs:6–45` create argument/group vectors across many consumers. The shared argument helper already filters comments. Rust may reuse a Vec allocation through in-place iteration, so this audit **does not assert two heap allocations** for every wrapper call. Instrument allocations before choosing an iterator/small-vector redesign.

Existing good guards: ExpectChange (`expect_change.rs:182`, `:254`) and PredicateMatcher (`predicate_matcher.rs:191`) avoid candidate rewrite parsing without a correction request; ReceiveMessages (`receive_messages.rs:300`) and ScatteredSetup (`scattered_setup.rs:380`) also gate full replacement construction. Keep these safeguards and follow their separation where metadata permits.

### F9 — many rules still enter callbacks before rejecting selector names

Only **two of 106** have interested_call_names in the audited snapshot: ExpectActual and HttpStatus. Most other rules reject quickly in check_node, but a large callback population remains. This is a common dispatch opportunity, not a claim that all remaining 104 rules support one fixed selector list or have expensive callbacks.

Derive routing from each callback's actual entry node. Example: Be examines the outer runner selector to/not_to/to_not, not just the visible offending be selector. Identifier pseudo-literals, declaration tokens, safe sends, operators, metadata nodes and configured names require their existing role handling. Focus/Pending already have early raw-name eligibility at this snapshot; the previous expensive pre-eligibility scope walk is **already fixed**. The separate queued 8873ea0 implements extra routing for those cops and NonNilCheck; it is not part of this audit snapshot and should not be duplicated.

## Practical next sequence

1. Profile current enabled-rule costs/counts before assigning a production speedup to these hypotheses.
2. Low-risk targeted candidates: ReturnFromStub early necessary conditions, prepared NoExpectationExample regexes, and no-sink description replacement work.
3. Cache description decode/interpolation facts and scope/sibling facts with explicit configuration/tree keys.
4. Tackle pairwise AST grouping with canonicalization and collision checks only after parity-sensitive normalization is specified.
5. Validate each change against original positive/negative/config cases, exact diagnostics/correctability, safe/unsafe bytes, final states and idempotence. Run the stored same-target repository comparison after a coherent implementation batch; do not retire rules, change enforcement or drop safety guards for timing.

The source checkout remained unchanged. No production-wide benchmark was run for this audit; no implementation is proposed as already validated.
