# Batch audit parity fixes

This change resolves the findings from the initial 346-event batch audit. The live batch now reports zero new discrepancies across all 98 added cops. Known native correction safeguards and upstream-invalid or looping corrections remain separately recorded.

## Changes

- Argument comments are excluded from semantic positional arguments in the eight affected rules. Corrections follow pinned Ruby's comment retention/deletion behavior. StringChars additionally preserves attached blocks, including their side effects and return values.
- DuplicateRequire and DuplicateCaseCondition compare decoded literal values. TripleQuotes handles mixed quote prefixes and decoded empty strings. RescueType recognizes character literals, and HashExcept handles character-literal keys.
- Encoded ClassEquality suggestions are diagnostic-only when unquoting them would produce invalid Ruby. Existing encoded-require and percent-BooleanSymbol safeguards remain in place.
- Shared ported highlights now use first-line Unicode codepoint lengths. Isolated AST endpoints match Ruby for assignments, rescues, `when` clauses, and send expressions. File-model checks use the same highlight calculation. These changes do not change correction spans.
- CompoundHash defaults to convention severity. Ported node-phase diagnostics honor explicit severity configuration.
- Top-level method and mixin checks recognize the appropriate parenthesized Ruby scope shapes. EmptyWhen ignores empty semicolon statements while retaining comment controls.

## Audit baseline corrections

The Ruby worker now enables correction by default when an example omits `AutoCorrect`, matching the native probe. Explicit disabled/contextual settings remain authoritative. This removes the KeywordParametersOrder false positives caused by inheriting the project's disabled correction setting only on the Ruby side.

The negated RegexpAsCondition case was verified against the actual Ruby CLI: repeated Ruby correction adds a second match operator and writes invalid syntax. The existing native single-pass output is explicitly recorded rather than copying that invalid follow-up.

A reference correction that produces invalid Ruby or an unchanged-source correction cycle can justify manual correctability only when the native source is unchanged, no native error occurred, and all other diagnostic metadata agrees. Range/severity differences, missing diagnostics, crashes, and internal errors remain failures. Regression tests enforce these boundaries.

## Verification

- 817 normal Rust tests pass; the explicit audit probe remains ignored by normal test runs.
- The full live audit selects 3,482 examples, with 3,443 compilable comparisons across all 98 cops and zero new discrepancies. Five invalid originals and 34 invalid mutations are separately recorded.
- 123 pinned semantic regression cases were added to the central corpus. Dedicated range tests cover 239 original audit cases and 20 additional controls.
- Runtime coverage is 11 explicit scenarios across 9 cops. Additional isolated tests cover 50 literal cases and the formerly unsafe StringChars block correction.
- Clippy retains 223 existing diagnostics with no additions. Formatting, whitespace, and function/module complexity checks pass.

Cache rules revision is 38. The [machine-readable result](batch-parity-fixes.json) records the batch comparison.

Logical highlight metadata is covered here. An additional bare-CR control agrees in logical column length but exposes a preexisting global terminal-rendering difference; grapheme/display-width rendering is outside this metadata fix.

The release binary passes 369 pinned-version CLI checks. Uncached before/after scans inspect the same 14,762-file Wistia corpus and retain identical 27 CLI offenses. These scans are correctness checks; no performance conclusion is drawn from one pair.
