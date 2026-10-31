# Wistia cop support

This branch implements the 79 cops marked **support** by the October 2026 Wistia
unsupported-cop audit. The registry contains 653 cops. The 24 Wistia, serializer,
Custom, and AnyCable rules are opt-in; they are not generic Ruby defaults.

The acceptance baseline is Wistia's Ruby 3.4.9 bundle: RuboCop 1.65.1,
Standard 1.40.0, rubocop-rails 2.26.0, and AnyCable Rails Core 1.6.2.

## Validation

- 615 Rust tests, including 1,278 pinned Ruby reference examples.
- Paired positive/negative Ruby fixtures for every new cop.
- Comparisons check offense locations, messages, correctability, and actual
  correction output. Configuration probes cover options, disabled corrections,
  safe corrections, binding scope, and AnyCable inclusion/exclusion. Unique short
  keys and explicitly required extension classes activate the intended policies.
- All 460 changed fixture outputs whose originals compile also compile with
  Ruby 3.4.9. Regexp and heredoc regressions include explicit semantic safeguards.
- `abcop --full` passes without threshold changes or suppressions.
- Full uncached Rust scan: 14,762 application files, 32.60 seconds, 179,224 KiB
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

No application files are rewritten by this change. The eight difficult audit
items and retained review candidates still require their Ruby coverage.

## Deliberate correction safeguards

Three reference examples record an explicit Rust difference and its reason:

- Inline free-spacing regexp whitespace classes retain their diagnostic but are
  not autocorrected when removing the brackets changes the match.
- FileWrite preserves an external heredoc once, where pinned Ruby duplicates it.
- FileRead preserves a heredoc filename body and terminator, where pinned Ruby
  drops them and produces invalid Ruby.

`Style/RandomWithOffset` uses arbitrary precision integers to preserve Ruby's
integer arithmetic, including hex/binary literals and values beyond `i64`.
`AutoCorrect: disabled` blocks both correction modes; `Safe` and
`SafeAutoCorrect` are respected by safe correction mode.

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
