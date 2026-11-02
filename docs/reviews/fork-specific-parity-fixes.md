# Fork-specific custom-rule parity fixes

This follow-up addresses the five reported fork-only findings. Argument comments affected four separate cops; the additional autoload require bypass is part of that shared cause. Behaviors that match pinned RuboCop 1.65.1 remain outside this change.

Implementation: `5689b14`.

## Changes

- `Cops/NoSidekiqReceive`, `Serializers/AvoidCapitalizedKeys`, `Serializers/AvoidMethodAndBlock`, and `Cops/NoRequireOnAutoLoadedPaths` skip comments before selecting their first argument. Previously a comment could hide a real offense.
- `AnyCable/StreamFrom` compares decoded static symbol values for `coder`, so `:"co\x64er"` and `%s[coder]` receive the same diagnostics as `:coder`. String and dynamic keys retain their existing behavior.
- `Lint/TripleQuotes` removes redundant empty string nodes from the original source while preserving intervening bytes. A continuation newline remains a newline, preserving interpolated `__LINE__`; the corrected source matches pinned Ruby.
- `Cops/NoUndiscoverableAutoLoadedConstants` uses structural constant names for definitions and enclosing namespaces. Valid multiline qualified class/module names no longer produce false positives, including root-qualified and nested forms. Existing fallback behavior remains for dynamic namespaces.

Cache rules revision is 35, invalidating older stored diagnostics.

## Verification

Fifty references refreshed against Wistia's bundle (Ruby 3.4.9, RuboCop 1.65.1) are included in the central parity suite:

| Fixture | Cases |
| --- | ---: |
| `custom-call-parity-regressions.json` | 25 |
| `autoload-qualified-names.json` | 16 |
| `triple-quote-gap-safety.json` | 9 |

They cover positive and negative diagnostics, source preservation for diagnostic-only rules, and TripleQuotes correction bytes. Annotated fixtures cover the original reproductions. Nine isolated Ruby comparisons preserve runtime output for the TripleQuotes cases, including continued and all-empty chains, interpolation, and ordinary controls.

All 794 Rust tests and 150 release CLI comparisons pass. The central suite contains 2,412 pinned references across the 98 added cops, plus 11 interpolation references. Changed Rust formatting, whitespace, and complexity checks pass (zero function/module ABC violations; 94 used-once advisories and no never-used warnings). Clippy has the same 223 preexisting diagnostics and no new ones.

Full uncached scans before and after the fixes inspect the same 14,762 files and produce identical 27 CLI offenses on the fixed Wistia corpus (`404394655f71e35560f1c24bf773af3de098c534`). This is a correctness check; the new edge-case regressions are exercised by the targeted references above.
