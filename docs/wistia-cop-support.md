# Wistia cop support

This branch implements the original 79 support candidates plus all 14 retained
review candidates from the October 2026 Wistia unsupported-cop audit. The registry
contains 667 cops. The 27 Wistia, serializer, Custom, and AnyCable rules are opt-in;
they are not generic Ruby defaults.

The acceptance baseline is Wistia's Ruby 3.4.9 bundle: RuboCop 1.65.1,
Standard 1.40.0, rubocop-rails 2.26.0, and AnyCable Rails Core 1.6.2.

## Validation

- 652 Rust tests, including 1,649 pinned Ruby reference examples across 93 added cops.
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

No application files are rewritten by this change. The 14 retained review
candidates below now have native implementations. Five difficult retained checks
still require Ruby: `Cops/NoUndiscoverableAutoLoadedConstants`,
`Lint/MixedRegexpCaptureTypes`, `Lint/OutOfRangeRegexpRef`,
`Lint/ShadowedArgument`, and `Style/EvalWithLocation`.
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
- `Style/MixinGrouping`
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
for that file while the custom cop correctly flags the sibling. The Wistia policy
branch adds a regression for this case; ordinary nested constants remain allowed.

## Deliberate correction safeguards

Reference cases record explicit Rust safeguards and their reasons:

- Inline free-spacing regexp whitespace classes retain their diagnostic but are
  not autocorrected when removing the brackets changes the match.
- FileWrite preserves an external heredoc once, where pinned Ruby duplicates it.
- FileRead preserves a heredoc filename body and terminator, where pinned Ruby
  drops them and produces invalid Ruby.
- `Lint/UselessTimes` refuses four pinned Ruby text substitutions that overwrite
  a reassigned block parameter, change string contents, rewrite a method name
  into an integer, or overwrite a shadowing block parameter. The findings remain
  visible; the incorrect rewrites are not advertised as correctable.

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
