# Custom-rule literal and scope parity fixes

This follow-up fixes the fork-specific findings reported in the fourth audit round, against Wistia's bundled Ruby 3.4.9 and RuboCop 1.65.1. Existing report-only safeguards remain in place.

Implementation: `6727a07`.

## File access

FileRead and FileWrite now prove the decoded mode is valid before offering a correction. Modes such as `"w'"` no longer become valid through raw quote stripping. The original ArgumentError behavior and existing file contents are preserved. Percent literals, escaped strings, and character modes receive the appropriate read/write/binread/binwrite correction. Argument matching also rejects extra modes and wrong block passes.

## Static literal classification and call diagnostics

A separate `ruby_static_string` helper follows the pinned parser's static-string AST classification for ordinary strings, character literals, and heredocs. It excludes interpolation and multi-part string nodes, including some static-looking multiline heredocs. The existing generic string decoder keeps its previous behavior. Dedicated parser-value references cover escapes, quoting, indentation, CRLF, continuation lines, and multiple pending heredocs.

AnyCable uses this classification for topics. Serializer checks recognize character/heredoc keys, respect the trailing forwarded-block option boundary, and use Ruby parent/descendant boundaries when consulting the method index. CSV.generate diagnostics highlight the send expression without its attached block; dedicated tests assert the complete range.

## Scope diagnostics

- Parameterized Class.new constructors no longer receive the special zero-argument namespace treatment that hid undiscoverable nested constants.
- Top-level side-effect diagnostics and MixinGrouping normalize direct parenthesized bodies according to Ruby's one-level begin-node behavior. Negative controls retain sibling and nested boundaries. MixinGrouping remains report-only.
- Spec constant exclusions distinguish ordinary describe blocks from numbered-parameter blocks. Nested lexical scopes and method selectors do not accidentally change the containing block's classification.

Cache rules revision is 37.

## Verification

- All 807 Rust tests pass. The central suite contains 2,654 pinned references across the 98 added cops, plus 11 interpolation references.
- Added 134 central-compatible cases: 45 call/literal/parent cases, 56 scope shapes, and 33 file modes. Four directive controls are separate because the raw Ruby Commissioner retains suppressed offenses. All 414 release CLI comparisons pass across the cases and correction modes.
- Eighteen additional parser-value references validate the new static-string helper. Supplemental tests check complete CSV highlight ranges and numbered-block behavior across the adaptive index threshold.
- Thirty-three tempfile runtime comparisons pass; all six wrong-quote cases preserve existing file contents. Existing write-correction safeguards remain in force.
- Changed Rust formatting and whitespace checks pass. ABC reports zero function/module violations and no never-used warnings; 97 used-once advisories remain. Clippy reports the same 223 preexisting diagnostics, with no new diagnostics.

Uncached scans before and after inspect the same 14,762-file Wistia corpus (`404394655f71e35560f1c24bf773af3de098c534`) and produce identical 27 CLI offenses. One run per binary took 20.110s before and 20.129s after; these are correctness checks rather than a performance estimate.
