# Wistia cop support

This branch implements all 98 retained support candidates from the October 2026
Wistia unsupported-cop audit: the original 79, the 14 retained review candidates,
and the final five larger checks. The registry contains 672 cops. The 28 Wistia,
serializer, Custom, and AnyCable rules are opt-in;
they are not generic Ruby defaults.

The acceptance baseline is Wistia's Ruby 3.4.9 bundle: RuboCop 1.65.1,
Standard 1.40.0, rubocop-rails 2.26.0, and AnyCable Rails Core 1.6.2.

## Validation

- 788 Rust tests, including 2,362 pinned Ruby reference examples across 98 added cops,
  plus 11 interpolation reference cases.
  The [parity and optimization follow-up](reviews/port-fixes-and-optimizations.md) records
  the 30 newly added regressions and correction-mode coverage. The
  [latest custom-rule fixes](reviews/custom-rule-optimization-fixes.md) record
  traversal/index/cache improvements. The [autoload and lazy-index follow-up](reviews/autoload-lazy-index-optimization.md)
  records the latest targeted timing and memory results, including the full-codebase trade-off.
- Paired positive/negative Ruby fixtures for every new cop.
- Comparisons check offense locations, messages, correctability, and actual
  correction output. Configuration probes cover options, disabled corrections,
  safe corrections, binding scope, and AnyCable inclusion/exclusion. Unique short
  keys and explicitly required extension classes activate the intended policies.
- All 591 changed fixture outputs also compile with
  Ruby 3.4.9. Regexp and heredoc regressions include explicit semantic safeguards.
- `abcop --full` passes without threshold changes or suppressions.
- Earlier pre-optimization uncached Rust scan: 14,762 application files, 32.60 seconds, 179,224 KiB
  peak RSS. It reports 27 existing regexp offenses (19 redundant escapes and
  8 single-element character classes).

The earlier clean Ruby run hid these regexp findings: the pinned
`Lint/DuplicateRegexpCharacterClassElement` shifts elements out of the shared
regexp parser tree. Later regexp cops therefore inspect an emptied tree. For
example, Ruby reports `/[a]/` with `Style/RedundantRegexpCharacterClass` alone,
but reports nothing with that lint cop running first. The new Rust rules retain
these diagnostics. They are independently confirmed by the pinned Ruby cops.

A Ruby `--only` run of the 79 rules took 263.90 seconds and reports 29 findings:
those 27 regexp issues plus two sleeps in `dev/spec/lanes_spec.rb`. Ruby's
`--only` forces those two findings despite that directory explicitly disabling
`Cops/NoSleep`. The Rust full scan honors that local override. A comparison using
Ruby's exclusion list instead of `--only` preserves configured enablement.
That run takes 257.88 seconds and exactly matches all 27 Rust diagnostics,
including locations and messages. All 24 app-specific rules also pass positive
activation probes using the real Wistia configuration.

No application files are rewritten by this change. All five formerly deferred
checks now have native implementations, leaving no active retained gaps in the
audited set. This coverage claim is scoped to that audit and Ruby 3.4, not every
RuboCop cop or Ruby configuration.
`Lint/NonDeterministicRequireOrder` and `Security/YAMLLoad` are inactive on Ruby 3.4.

## Retained review candidates now supported

- `Cops/NoSetex`
- `Lint/RefinementImportMethods`
- `Lint/UselessTimes`
- `Lint/NumberedParameterAssignment`
- `Migration/DepartmentName`
- `Serializers/PreferAttributeBlockOverAttributeAndMethod`
- `Serializers/PreferAttributeBlockOverAttributesAndMethods`
- `Style/ColonMethodDefinition`
- `Style/EmptyBlockParameter`
- `Style/KeywordParametersOrder`
- `Style/MixinGrouping` — diagnostic-only in both styles, including `-A`.
- `Style/MultilineWhenThen`
- `Style/TrailingBodyOnMethodDefinition`
- `Style/TrailingMethodEndStatement`

The additional reference cases cover both mixin styles, serializer scope isolation,
commented arguments, keyword defaults and disabled corrections, endless and
singleton methods, short directive names, and actual corrections. Reserved
numbered bindings (`_1` through `_9`) are rejected by the syntax gate on Ruby 3;
the lint rule also diagnoses valid but confusing names such as `_0` and `_10`.
The syntax gate covers assignments, loops, rescue bindings, patterns, explicit
parameters, and reserved method names. Short disable/enable/todo names resolve to
the same qualified policy used by config loading. Known reference names are
resolved even when their actual check is not yet ported; name resolution is not
an implementation claim. The reference refresh script also regenerates this
name inventory and the pinned legacy-name inventory.

The serializer method-to-block rules are now diagnostic-only even under explicit
correction enablement, because moving public methods cannot generally preserve
callers, parameters, and control flow. Keyword-order enforcement continues to honor
Wistia's `AutoCorrect: disabled` setting. All 14 positive activation probes pass using the real
Wistia config without forcing enablement via `--only`. The 93 changed Ruby outputs
from this batch compile under Ruby 3.4.9. Eight engine probes also verify disabled
corrections in both modes, non-default indentation width, and the setex rewrite.

Both implementations inspect the same 14,762 application files and find zero
offenses from the new 14 rules. The full Rust scan takes 35.09 seconds and retains
the same 27 pre-existing regexp diagnostics, with unchanged locations and messages.
The uncached Ruby comparison of these 14 configured rules takes 248.95 seconds.
These runs have different rule scopes; the Ruby timing is not a full-suite timing.

## Final five retained checks

- `Cops/NoUndiscoverableAutoLoadedConstants` checks classes, modules, and constant
  assignments against autoload roots and namespace prefixes. It handles concerns,
  STI roots, project acronyms such as HMS/LLM, qualified definitions, and sibling
  constants. Literal acronym declarations are parsed from the inflection file;
  no Rails boot or arbitrary Ruby execution is required.
- `Lint/MixedRegexpCaptureTypes` counts named and numbered groups while respecting
  escaped delimiters, character classes, inline comments, and extended modes.
- `Lint/OutOfRangeRegexpRef` tracks capture availability through the pinned Ruby
  callback order, matching operations, blocks, case clauses, and pattern clauses.
  Dynamic interpolation leaves counts unknown. The inherited `Safe: false`
  setting is retained: Ruby's source-order analysis can flag conditional matches
  conservatively. This rule never rewrites code.
- `Lint/ShadowedArgument` uses the shared scope model, including original argument
  references, conditional/block assignments, destructuring, and implicit
  `binding`/`super` reads under both `IgnoreImplicitReferences` settings. It never
  rewrites code.
- `Style/EvalWithLocation` checks literal code passed to eval-family methods and
  supplies accurate filename/line information where correction is safe. It handles
  string and heredoc line offsets, including nested pending heredocs. It does not
  invent an eval binding. Effectful filename/line expressions and uncertain
  argument shapes keep diagnostics without unsafe corrections.

The 375 additional examples include 41 eval cases reviewed for correction safety.
They run both directly and through the real syntax/scope/configuration pipeline.
All five checks activate under the real Wistia config without forcing `--only`.
Four are diagnostic-only under both correction modes, even with correction
explicitly enabled. Four runtime backtrace probes verify the eval filename and
line after correction; two additional probes preserve missing-binding/effectful
argument cases. All 585 changed reference outputs compile under Ruby 3.4.9.

Ruby's shorthand regexp interpolation helper misses `#$value` as dynamic input;
Rust explicitly treats its capture count as unknown, like `#{$value}`, to avoid
false claims. These two intentional diagnostic differences are documented in the
reference snapshot. Ordinary pinned Ruby locations/messages/correctability and
safe correction output match the expanded fixture set.

The final full uncached Rust scan inspects 14,762 files in 50.04 seconds
(180,884 KiB peak RSS). It reports the same 27 pre-existing regexp findings with
unchanged locations and messages, and zero findings from the new five checks.
An uncached Ruby comparison selecting these five configured cops inspects the same
files and reports zero findings in 273.75 seconds (161,692 KiB peak RSS). The two
timings have different rule scopes; the Ruby figure is not a full-suite timing.

## Follow-up policy review

Wistia retired `Lint/DuplicateRegexpCharacterClassElement`,
`Style/RedundantCondition`, and `Style/SuperArguments`. The first provides modest
syntax cleanup and has the shared-tree mutation bug described above; the other
two mainly prefer shorter code or implicit argument forwarding. A follow-up
probe confirmed that retained `Lint/MixedCaseRange` also consumes the shared
regexp tree through `RegexpRanges#populate`, so full Ruby runs continue hiding
the 27 findings. Temporarily excluding that second cop for diagnosis restores
all 27 Rust-reported locations; retiring the duplicate cop alone does not repair
the shared-parser problem.

Keep `Lint/MixedRegexpCaptureTypes`, `Lint/OutOfRangeRegexpRef`,
`Lint/ShadowedArgument`, and `Style/EvalWithLocation` for their correctness and
debugging value. Retain the existing `Safe: false` setting on out-of-range refs.

The autoload overlap review also retains
`Cops/NoUndiscoverableAutoLoadedConstants`. Zeitwerk eager loading verifies each
file's expected constant, but an unrelated sibling defined in the same file can
still lack its own autoload entry. An isolated loader probe passes eager loading
for that file while the custom cop correctly flags the sibling. Both the Wistia
policy branch and the native port have regressions for this case; ordinary nested
constants remain allowed.

## Deliberate correction safeguards

Reference cases record explicit Rust safeguards and their reasons:

- Inline free-spacing regexp whitespace classes retain their diagnostic but are
  not autocorrected when removing the brackets changes the match.
- FileWrite preserves an external heredoc once, where pinned Ruby duplicates it.
- FileRead preserves a heredoc filename body and terminator, where pinned Ruby
  drops them and produces invalid Ruby.
- `Lint/UselessTimes` and `Style/MapCompactWithConditionalBlock` are report-only
  in both correction modes, preserving return values, bindings, and collection
  semantics. `Style/HashExcept` leaves dynamic key expressions and complex
  literal collections manual; supported static literals retain unsafe corrections.

`Style/RandomWithOffset` uses arbitrary precision integers to preserve Ruby's
integer arithmetic, including hex/binary literals and values beyond `i64`.
`AutoCorrect: disabled` blocks both correction modes; `Safe` and
`SafeAutoCorrect` are respected by safe correction mode.

## Custom cop review fixes

The [custom cop review](reviews/custom-cops.md) records the complete findings and
resolution. Multi-edit corrections now have stable ownership, respect disable
comments, apply atomically, and receive accurate correction credit. Invalid native
syntax in any candidate batch aborts without writing the original file. Independent
same-position insertions are serialized across correction passes.

Migration correctors insert into the argument list, preserve supplied shorthand
options, and decline unknown option bundles. NoSetex only reorders stable literals
or proven lexical locals; uncertain/effectful arguments remain manual findings.
The serializer transformation code was removed while retaining both checks.
The diagnostic guardrail fixes cover Redis transactions, protected-write arguments,
AnyCable option boundaries, and AST-based rescue reporting. Existing Ruby matcher
holes and unsafe rewrites are documented as intentional reference differences.

The post-fix full scan inspects 14,762 files in 39.93 seconds and reports the same
27 regexp findings with unchanged locations and messages. All 554 changed reference
outputs compile on Ruby 3.4.9. Runtime probes preserve five serializer field results
and three Redis/table argument results. These checks supplement the clean current
corpus with adverse correction inputs that it does not contain.

## Reproducing the reference fixtures

The reference snapshot is `tests/fixtures/ported-reference.json`. Source examples
come from the pinned RuboCop specs ([MIT license](licenses/rubocop.txt)), Wistia's custom cop specs, and
focused regression probes. Ported Ruby/AnyCable/Rails policies retain their
upstream licenses under `docs/licenses/`. Upstream-derived cases retain their spec origin.
The annotated `tests/fixtures/cops/` files are checked alongside the snapshot;
multiline diagnostic messages are retained in full in the JSON reference.

From the Wistia checkout with its installed bundle, run:

```sh
bundle exec ruby /path/to/rrubocop/scripts/refresh_ported_reference.rb .rubocop.yml
```

This regenerates Ruby expectations, including real rewrites; it does not lint or
rewrite application files. The autoload lookup uses the isolated
`tests/fixtures/ported_project` root for both implementations. General cop
configuration comes from the supplied project config. Rust tests do not require
Ruby or the Wistia checkout.

```sh
cargo test --locked
abcop --full
```

## Implemented inventory

### AnyCable (3)

- `AnyCable/InstanceVars`
- `AnyCable/PeriodicalTimers`
- `AnyCable/StreamFrom`

### Cops (17)

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
- `Cops/NoSidekiqReceive`
- `Cops/NoSleep`
- `Cops/NoTopLevelSideEffectsInAutoloadedFiles`
- `Cops/RescueMustReportError`

### Custom (1)

- `Custom/RequireRespondToAny`

### Lint (30)

- `Lint/AmbiguousAssignment`
- `Lint/BooleanSymbol`
- `Lint/DeprecatedConstants`
- `Lint/DuplicateCaseCondition`
- `Lint/DuplicateElsifCondition`
- `Lint/DuplicateRequire`
- `Lint/DuplicateRescueException`
- `Lint/EmptyExpression`
- `Lint/EmptyWhen`
- `Lint/ErbNewArguments`
- `Lint/FloatComparison`
- `Lint/IdentityComparison`
- `Lint/InterpolationCheck`
- `Lint/MultipleComparison`
- `Lint/NestedPercentLiteral`
- `Lint/OrAssignmentToConstant`
- `Lint/RaiseException`
- `Lint/RedundantWithIndex`
- `Lint/RedundantWithObject`
- `Lint/RegexpAsCondition`
- `Lint/RescueType`
- `Lint/ReturnInVoidContext`
- `Lint/SafeNavigationConsistency`
- `Lint/SafeNavigationWithEmpty`
- `Lint/SelfAssignment`
- `Lint/TopLevelReturnWithArgument`
- `Lint/TrailingCommaInAttributeDeclaration`
- `Lint/TripleQuotes`
- `Lint/UriEscapeUnescape`
- `Lint/UriRegexp`

### Rails (1)

- `Rails/ReversibleMigrationMethodDefinition`

### Security (1)

- `Security/CompoundHash`

### Serializers (3)

- `Serializers/AvoidCapitalizedKeys`
- `Serializers/AvoidKeyAndBlock`
- `Serializers/AvoidMethodAndBlock`

### Style (23)

- `Style/ClassEqualityComparison`
- `Style/Dir`
- `Style/FileRead`
- `Style/FileWrite`
- `Style/HashConversion`
- `Style/HashExcept`
- `Style/MapCompactWithConditionalBlock`
- `Style/MixinUsage`
- `Style/OrAssignment`
- `Style/RandomWithOffset`
- `Style/RedundantAssignment`
- `Style/RedundantConditional`
- `Style/RedundantFileExtensionInRequire`
- `Style/RedundantRegexpCharacterClass`
- `Style/RedundantRegexpEscape`
- `Style/RedundantSort`
- `Style/SlicingWithRange`
- `Style/StderrPuts`
- `Style/StringChars`
- `Style/TopLevelMethodDefinition`
- `Style/TrailingCommaInBlockArgs`
- `Style/UnlessLogicalOperators`
- `Style/UnpackFirst`

The [configuration, CodeMap, and custom dispatch follow-up](reviews/config-codemap-custom-optimizations.md) records the latest four optimizations and full-corpus measurements.

The [correctness and correction-indexing follow-up](reviews/correctness-and-correction-indexes.md) records Unicode, directive, interpolation, and custom-rule fixes, correction scaling measurements, and the conservative block-comment reference exception.

The [review correctness and memory follow-up](reviews/review-correctness-and-memory-fixes.md) fixes literal-delimiter and directive regressions and records lower allocation overhead, 90 Ruby reference runs, and targeted memory measurements.

The [remaining custom-rule safety and efficiency fixes](reviews/remaining-custom-safety-efficiency.md) add decoded literal/constant parity, conservative retained-cop rewrites, shared source excerpts and compact cache storage, with runtime checks and measured tradeoffs.

The [retained autocorrection guard fixes](reviews/retained-autocorrect-guard-fixes.md) cover ten rules, make MixinGrouping report-only, and add 100 targeted cases with release/runtime verification.

The [return-value and key-eligibility safeguards](reviews/unsafe-value-eligibility.md) disable Times and MapCompact corrections and constrain HashExcept key rewrites.
