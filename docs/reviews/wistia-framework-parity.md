# Framework cop parity implementation

This branch owns Rails, Migration, Performance and FactoryBot. The baseline
inventory has 90 enabled cops, including 67 no-op stubs. The first batch replaces
7 stubs with implementations; 60 stubs and the remaining implemented-cop gaps
still need work. The exhaustive queue is `wistia-framework-parity.json`.

## Original-rule review

Reference: Ruby RuboCop 1.65.1, rubocop-rails 2.26.0 and
rubocop-performance 1.21.1 from Wistia's locked bundle. Each implementation was
reviewed against the original Ruby source after porting, not inferred from a
few examples. `upstream_source` in the JSON inventory records the source file.

- `Rails/Blank`: nil/empty combinations (`nil?`, `== nil`, `nil ==`, negation,
  double-negated empty), negated present, implicit receivers, `unless` statement
  and modifier, all three configuration switches, safe-navigation exclusions,
  direct `blank?` method-body exclusion and nested-method negative controls.
- `Rails/Present`: plain/nonnull/double-negated receivers followed by negated
  empty, negated blank, implicit receivers, both `unless` forms, all three
  switches, safe-navigation negatives.
- `Rails/OutputSafety`: html_safe, receiverless raw, safe_concat, literal versus
  interpolated strings, nested I18n translations and non-I18n receivers.
- `Performance/Count`: select/filter/find_all/reject, count/length/size, blocks,
  symbol block passes, safe navigation, counted arguments, attached outer blocks,
  multi-statement blocks, comments, Unicode and raw-byte preservation.
- `Performance/Size`: array/hash literals, to_a/to_h, Array/Hash constructors,
  constructor arity, blocks and count arguments.
- `Performance/RangeInclude`: inclusive/exclusive literal ranges, member?/include?,
  safe navigation and one versus two parenthesis layers. String ranges retain
  the original unsafe-correction classification.
- `Performance/UriDefaultParser`: rooted/unrooted URI::Parser, options, unrelated
  namespaces, safe navigation and attached blocks. Correction preserves the block.

Post-review caught and fixed safe-navigation false positives, Hash constructor
arity, accidental URI block deletion, multiline/Unicode highlight lengths,
misreported disabled-autocorrection metadata, and raw-byte source loss.

## Evidence and intentional differences

The strict differential driver compares original diagnostic count, location,
message, severity, highlight length and correctability; safe/all corrected bytes;
Ruby compilation; final-state specimens; and second-pass idempotence. It imports
production cop options from Wistia's original Ruby configuration and normalizes
fixture paths. Scope-sensitive end-to-end CLI checks remain follow-up work.

Current matrix: 111 original specimens and 222 second-pass specimens. Six ordinary
implemented cops have no observed differences. The strict run intentionally
**fails** with six events for one `Performance/Count` source repeated in the
original and second-pass matrices: `items.reject(&predicate).count`.
Ruby rewrites it as `items.count { !predicate.call }`, dropping the block argument.
Rust diagnoses the offense but declines that correction and reports it as not
correctable. This is a visible safety difference, not an allowed exception or a
claim of complete parity. The driver contains no difference allowlist.

Additional source-preservation guards decline corrections that would remove an
inter-call or inter-predicate comment. Rust tests assert those guards directly.
Count correction uses independent token edits, preserving raw bytes and comments
inside the filter block; predicate corrections preserve the receiver's original
bytes rather than rebuilding it through a lossy UTF-8 conversion.

These are finite probes, not a proof of equivalence. Further cases are still
needed for uncommon implicit bindings, arbitrary syntax equivalence, cross-cop
interactions, Rails version overrides and disabled `Style/UnlessElse` interaction.
The existing Ruby unsafe classifications remain important: correcting Blank on
whitespace strings, Count on ActiveRecord relations, or RangeInclude on string
ranges can intentionally change behavior even when Rust matches Ruby.

## Validation

```sh
CARGO_TARGET_DIR=/dev/shm/rrubocop-framework-target CARGO_PROFILE_DEV_DEBUG=0 \
  CARGO_INCREMENTAL=0 cargo test --locked --lib cop::performance
CARGO_TARGET_DIR=/dev/shm/rrubocop-framework-target CARGO_PROFILE_DEV_DEBUG=0 \
  CARGO_INCREMENTAL=0 cargo test --locked --lib cop::rails
python3 scripts/framework_parity/check.py \
  --ruby-project /path/to/wistia --output /tmp/framework-parity
```

The differential command exits nonzero for every mismatch, including the explicit
upstream proc hazard above. Its `failures.json` and `summary.json` retain the full
evidence. Source fixture tests and safety regression tests are ordinary Rust tests.
No Wistia policy changes or rule retirements are included in this branch.
