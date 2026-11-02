# Custom-rule parity follow-up

This change addresses the fork-specific findings from the next custom-rule audit, using Wistia's bundled Ruby 3.4.9 and RuboCop 1.65.1 as the baseline. It does not change the upstream-identical hazards excluded by the user.

Implementation: `f1fe97e87c6144a9579feddbb287cedd36153f61`.

## Literal matching and corrections

`Style/RedundantFileExtensionInRequire` matches decoded string values rather than stripping arbitrary quote characters. A path such as `'json.rb"'` has no `.rb` suffix and is preserved. Ordinary and percent-quoted literals with a real suffix receive the pinned correction. Encoded suffixes remain manual when raw suffix deletion could damage an escape sequence or percent-literal delimiter.

`Lint/BooleanSymbol` matches decoded static symbols, preserving opposite-quote characters that belong to the symbol value. Quoted symbol corrections retain the pinned source form. Percent-symbol corrections that loop upstream remain manual.

## Custom call diagnostics

Serializer checks distinguish the presence of a `key:` pair from the presence of an explicit value node. Shorthand keywords no longer trigger fallback to the attribute name or bypass the key/block check.

`Cops/RescueMustReportError` locates the actual rescue keyword token. Text containing `rescue` in a preceding string no longer moves the offense to the wrong line or prevents an inline disable from applying.

AnyCable callback matching follows the pinned argument shapes and preserves distinct callback and coder diagnostics, rather than treating any lambda argument as a callback.

## Scope diagnostics

`Cops/NoConstantDefinitionsInSpecs` checks constant targets in multiple/operator assignments and loop targets while retaining describe-group exclusions and per-target locations.

`Rails/ReversibleMigrationMethodDefinition` matches structural superclass and version-index shapes instead of raw prefixes, excluding lookalike constants and recognizing multiline names.

`Cops/NoTopLevelSideEffectsInAutoloadedFiles` recognizes allowed ternary definition/require branches and explicit equality-call script guards.

Cache rules revision is 36.

## Verification

- All 799 Rust tests pass. The central suite now has 2,520 pinned references across the 98 added cops, plus 11 interpolation references.
- Added 108 pinned cases: 33 custom runtime shapes, 48 scope cases, and 27 literal/suffix cases. These cover diagnostics and correction bytes across Off, Safe, and All modes. Rescue suppression is additionally checked in the real directive pipeline.
- All 27 isolated Ruby literal/runtime checks pass. Source-preserving exceptions compare against the unchanged original; supported corrections compare against pinned Ruby.
- Changed Rust formatting and whitespace checks pass. ABC reports zero function/module complexity violations, zero never-used warnings, and the same 94 used-once advisories.
- Clippy remains blocked by the same 223 baseline diagnostics, with no new diagnostics.

All 324 release CLI checks pass across the 108 new cases and three modes.

Uncached before/after scans of the fixed Wistia corpus (`404394655f71e35560f1c24bf773af3de098c534`) inspect 14,762 files and retain identical 27 CLI offenses. The edge-case fixes are exercised by the new targeted references above.
