# Correctness fixes and correction indexing

Follow-up to the read-only configuration, traversal, and custom-rule audits. Baseline: `448212203bbd0a72c50082687682bb097ada9d18`; implementation: `1c9b823ee7a65689913aa6b35373df0eda9850f3`.

## Correctness

- Unicode columns now convert back to byte offsets correctly, so multiple trailing-whitespace offenses after multibyte characters retain correction owners.
- Disable directives come from actual Ruby comment nodes. Heredoc, multiline string, and regex contents cannot suppress offenses. Directive-checking cops share the comment information so their corrections cannot rewrite literal content. Real comment prefixes are isolated from directive-looking strings earlier on the same line.
- CodeMap exposes embedded Ruby inside interpolated strings, regexes, percent literals, and heredocs while protecting literal text and interpolation delimiters. Nested literals remain protected.
- Sidekiq worker detection recognizes rooted and multiline constant spellings; its raw negative marker is conservative.
- Protected-write and Redis-expiration checks compare decoded static symbol values, covering hex, octal, Unicode, quoted labels, percent symbols, and line continuations. Interpolated symbols are not evaluated.
- Single-cop redundant-disable offenses now highlight the actual directive rather than just the cop token, matching Ruby locations.
- New interpolation tests exposed a preexisting ownership boundary bug: zero-width edits immediately after a highlighted offense could lose ownership when multiple offenses existed. Endpoint insertions now retain owners; non-insertion matching keeps its previous boundaries.

## Optimizations

ASCII coordinates use constant-time conversion; Unicode lines lazily cache codepoint boundaries for both conversion directions.

Correction owners are grouped once by cop and resolved with a sorted candidate sweep. Accepted-edit credits are indexed by cop/owner and merged intervals. Conflict selection indexes accepted ranges and insertion positions, retaining grouped atomicity, duplicate aliases, insertion boundaries, and stable ordering. This removes the repeated whole-list scans in ownership, reconciliation, and selection.

Parsed directives are cached per source snapshot and use the existing parse tree in the engine. Files without a directive marker skip comment traversal. Pre-filter diagnostic cloning runs only when redundant-disable auditing needs it.

Source cops explicitly request CodeMap construction. All nine consumers opt in; map-independent checks avoid building ranges. Broader dispatch-plan caching, require-cache concurrency, and small Redis/migration micro-optimizations remain unimplemented pending evidence of value.

Cache rules revision advances to 30. No application source files were modified.

## Reference exception

Ruby 1.65.1 treats a `# rubocop:disable` line inside an `=begin` block comment as a redundant directive and can rewrite the block comment during autocorrection. Rust deliberately ignores those embedded markers and preserves the comment bytes, while still correcting real trailing whitespace outside the block. This conservative exception is recorded in the 33-run comparison; it is not claimed as exact reference parity. Multi-cop redundant-directive ranges retain existing behavior.

## Validation

- 748 Rust tests pass across all targets. The final Clippy-only conditional simplification also passes the 17 focused redundant-disable tests.
- The original reference corpus now includes 2,070 cases plus 30 earlier parity regressions (2,100 across the 98 ports); 46 unique new custom cases were added after replaying 47 examples through the project's bundled Ruby cops. Eleven additional interpolation references run through Off/Safe/All modes with exact offense ranges and corrected bytes.
- Selection preserves the prior implementation across 20,736 four-edit combinations and 2,000 longer mixed-owner sequences. Ownership covers 8,000 candidate combinations; credit tests cover aliases and inclusive boundaries.
- Changed Rust formatting and ABC gates pass: zero function/module complexity offenses, 76 used-once advisories, zero never-used warnings. Clippy remains blocked by preexisting diagnostics; comparison finds no new diagnostic groups and removes four existing findings.
- Seven regression `.rb` fixtures intentionally contain trailing whitespace. Diff checks pass when those expected fixture bytes are excluded; production source has no whitespace errors.

The CLI comparison retains identical complete diagnostic arrays on 67 unchanged inspection probes. Of 21 earlier correction comparisons, 13 remain identical and eight intentionally improve because endpoint insertions now retain owners. All 18 comma-check Off/Safe/All results match bundled Ruby offenses and corrected bytes; the three nested-option comparisons remain identical. The separate directive comparison matches all 30 ordinary/literal/interpolation/Unicode/comma runs, including locations and corrected bytes; its three block-comment exceptions are described above.

## Measurements

Same release profile and fixed Wistia corpus, sequential before/after processes with warm filesystem. Three alternating repetitions per binary for synthetic scopes and the 632-file sample; two per binary for the full corpus, ordered before/after/after/before. No builds or agent scans ran during timings.

`--cache false` skips reads but still writes. Sample/full runs use separate cache directories per binary and scope, staying below the pruning cap. Synthetic scopes use an intentionally unavailable cache backend. Corrected sources are reset before each correction run. Elapsed time includes startup, parsing, correction iterations, JSON output, and rewriting.

| Scope | Before median (s) | After median (s) |
| --- | ---: | ---: |
| correct-2000 | 0.138207 | 0.087521 |
| correct-8000 | 1.059779 | 0.320947 |
| correct-16000 | 3.746320 | 0.642320 |
| map-independent-32000 | 0.211784 | 0.195967 |
| wide-comma-4000 | 0.333460 | 0.163678 |
| wide-comma-16000 | 4.068975 | 0.881018 |
| sample | 2.638780 | 2.642921 |
| full | 37.490304 | 38.047431 |

The full corpus retains identical complete diagnostic arrays (14,762 files, 27 offenses). Elapsed median changes by +1.49%; aggregate user CPU changes by +0.38%. This pass provides strong correction-scaling improvements, not an additional demonstrated whole-codebase speedup. Two full runs per binary cannot establish the cause of small wall-time differences. Timings from the earlier optimization report were collected under different load and should not be compared directly with this batch.

`correct-N` fixes N ordinary trailing-whitespace lines; `wide-comma-N` fixes N spaces before commas on one long literal-array line. All paired synthetic offense arrays and corrected bytes match. The 16,000-comma stress input reaches roughly 0.8 GiB peak RSS in both binaries due to the large parse/correction workload; it is a scaling probe, not typical application input. `map-independent-32000` selects only OutOfRangeRegexpRef on an array of 32,000 strings with no regexp-reference marker. Its gain is modest because parsing remains necessary.

[Raw timings, provenance, diagnostic hashes, reference comparisons, and runner](correctness-and-correction-measurements.json).

The [subsequent review fixes](review-correctness-and-memory-fixes.md) address percent-literal delimiters, CRLF removal, multi-cop highlights, selector re-enables, and allocation overhead found during independent review.
