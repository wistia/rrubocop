# Framework cop parity implementation

This branch owns 90 enabled Rails, Migration, Performance and FactoryBot cops.
The baseline inventory contained 67 no-op stubs. The first nine batches replace
24 stubs and repair an existing ReverseEach detector/corrector; 43 stubs and the remaining implemented-cop gaps
still need work. `wistia-framework-parity.json` tracks every owned rule.

## Original-rule review

Reference: Ruby RuboCop 1.65.1, rubocop-rails 2.26.0,
rubocop-performance 1.21.1 and rubocop-factory_bot 2.26.1 from Wistia's locked
bundle. Each port was reviewed against the original Ruby algorithm after writing
it. The inventory records each original source file.

- `Rails/Blank`: nil?/empty?, equality with nil in both orders, negation,
  double-negated empty, negated present, implicit receivers, both unless forms,
  configuration switches, safe-navigation exclusions and defining-blank exclusion.
- `Rails/Present`: plain/non-null/double-negated receivers with negated empty,
  negated blank, implicit receivers, both unless forms and configuration switches.
- `Rails/OutputSafety`: html_safe, receiverless raw, safe_concat, literal versus
  interpolated strings, nested I18n translations and non-I18n receivers.
- `Performance/Count`: select/filter/find_all/reject; count/length/size; blocks,
  symbol block passes, safe navigation, outer arguments/blocks, multiple block
  statements, comments, Unicode and raw bytes.
- `Performance/Size`: array/hash literals, to_a/to_h, Array/Hash constructors,
  constructor arity, blocks and count arguments.
- `Performance/RangeInclude`: inclusive/exclusive literal ranges,
  include?/member?, safe navigation and parenthesis depth.
- `Performance/UriDefaultParser`: rooted/unrooted constants, constructor options,
  unrelated namespaces, safe navigation and attached blocks.
- `FactoryBot/FactoryClassName`: rooted/namespaced constants, Hash/OpenStruct
  exemptions, receiver forms, option hashes, symbol/string keys and correction policy.
- `FactoryBot/AttributeDefinedStatically`: factory/trait/transient blocks,
  reserved methods, implicit/self/proxy receivers, dynamic attributes, associations,
  proc arguments, bare hashes, parentheses and raw-byte-preserving wrappers.
- `FactoryBot/CreateList`: times/times.map/Array.new, repeated arrays,
  explicit receivers, both styles, ExplicitOnly, options, dynamic calls versus
  lexical locals, index usage, nested factory blocks and integer source forms.

The distinct post-port review fixed safe-navigation false positives, Hash arity,
URI block deletion, multiline/Unicode highlight lengths, correction-policy
metadata, FactoryBot argument parentheses, lexical locals and raw-byte source loss.

## Evidence

The strict driver compares offense counts, positions, messages, severity,
highlight length and correctability; safe/all corrected bytes; compilation;
final-state specimens; and second-pass idempotence. Original cop options come
from Wistia's effective Ruby configuration. Fixture scopes are normalized;
end-to-end checks of actual configured file scopes remain pending.

The current matrix has **295 original specimens**, **590 second-pass specimens**
and **20 explicitly authored trusted runtime programs**. Ordinary fixture code
is parsed/compiled, never executed. Ten implemented cops have no observed
upstream differences in this bounded matrix. Eight safety policies/upstream hazards intentionally
differ and remain visible in every strict run; the command exits nonzero for
all differences and has no allowlist.

### Count proc argument safeguard

Ruby rewrites `items.reject(&predicate).count` to
`items.count { !predicate.call }`, dropping the element argument. A trusted
lambda requiring one argument proves the Ruby correction raises an arity error.
Rust diagnoses the offense but declines this correction and preserves execution.
Four source specimens repeated through the matrices produce **24 differences**
(12 correctability reports and 12 unsafe byte outputs), plus **3 runtime
differences**. The extra trusted programs demonstrate that deleting the outer
count changes a block break value, and negating only the final reject expression
does not invert an earlier next value. Rust declines those control-flow rewrites. The guard is deliberately not described as exact correction parity.

### Present safety classification

Pinned `Rails/Present#on_and` invokes `exists_and_not_empty?`, then replaces
`receiver && !receiver.empty?` with `receiver.present?` without a receiver-type or
value-context guard. The original configuration advertises safe autocorrection.
Actual production-config Ruby `-a` probes confirm:

```ruby
require 'active_support/core_ext/object/blank'
value = nil
p(value && !value.empty?) # prints nil; corrected source prints false
```

For `value = " "`, the original prints true and the corrected version false.
This is more than a hypothetical edge: nil return values and whitespace
truthiness differ under the original rule's global matcher. Negated blank and
unless forms have stronger method-contract equivalence, but the current engine
classifies safety per cop. The conservative Rust classification therefore
requires **explicit unsafe correction for the entire Present cop**. Detection
and `-A` bytes still match the reference in the tested cases; `-a` preserves the
source instead of silently making a value-changing rewrite.

The stricter default produces **22 safe-mode byte differences** in the repeated
matrix and **1 safe runtime difference** from Ruby. The trusted runtime report
also records that `-A` matches Ruby while changing the original nil value. This
is an explicit policy/safety difference, not a semantics-preservation claim.

### Additional reviewed rules and upstream hazards

The third batch implements `Performance/FixedSize` (literal types, splats,
constants, blocks and count arguments), `Performance/FlatMap` (map/collect,
block-pass, explicit levels, no-parameter policy and safe navigation),
`Rails/ArelStar` (literal decoding, receiver forms, lexical shadowing and original
unsafe metadata), `Rails/FindBy` (take versus first policy, safe navigation and
query arguments), and `Rails/Pick` (arguments, safe navigation and Rails6 gate).
Word/symbol array literals are also recognized by Size and FixedSize. Shared
cached local-binding classification replaces the temporary factory lexical scan.

- FlatMap refuses bang flatten and block-break rewrites that change return values.
  Two trusted programs preserve the original values in Rust while Ruby changes
  them in both correction modes. This contributes **22 events**: 18 diagnostic/
  byte differences across repeated specimens and four runtime differences.
- Original FindBy turns `User.where(id: 1).take()` into invalid
  `User.find_by(id: 1)()`. Native correction is declined rather than generating
  invalid syntax or returning a lint error. This contributes **9 events**.
- Original FindBy raises NoMethodError on `User.where { |x| x.active? }.take`;
  native safely skips that unsupported receiver. The three repeated Ruby errors
  remain explicit audit failures, not hidden exceptions.
- Original Pick similarly produces invalid `User.pick(:name)()` for
  `User.pluck(:name).first()`. Native diagnoses but declines the correction;
  this contributes **6 events**.

### Fourth batch and shared syntax fixes

Caller now implements receiver/arity checks, cached local-binding disambiguation,
arbitrary-size integer offsets, numeric bases, first/index forms and token ranges.
InefficientHashSearch implements key/value searches, correct unsafe metadata,
safe navigation and original argument formatting. ReverseEach's pre-existing
implementation now has the original return-use exclusions, selector range,
argument checks, safe-navigation behavior and actual correction support.

Trusted programs expose two additional original safe-correction hazards:

- Caller negative indexing selects the last frame, while rewriting it as a
  bounded nonnegative-offset lookup selects a different frame. Rust diagnoses
  but declines that correction: **11 events**, including two runtime comparisons.
- ReverseEach used as a method's implicit return changes a reversed array into
  the original array. Rust preserves the implicit return: **11 events**, including
  two runtime comparisons.
- URI parser construction with comment-only parentheses is reported but not
  rewritten, because replacing the constructor removes its comments. The strict
  matrix explicitly records **9 events** for that source-preservation difference.

The shared argument helper now excludes syntax comments, matching Ruby AST
semantics; two parser regression tests cover comment-bearing and comment-only
argument lists. The complete native suite passes after this shared change.
A small send-range helper excludes attached blocks/trivia instead of guessing at
whitespace, retaining block comments. OutputSafety now distinguishes bare I18n
calls from shadowed locals and ignores an attached block when searching the
original send expression. Factory option keys use the existing Ruby literal
decoder, including quoted-symbol labels and escaped keys.

### Source-preservation guards

Corrections that would discard inter-call/inter-predicate comments are declined.
Count uses independent token edits, retaining comments and raw bytes inside the
filter block. Predicate rewrites preserve original receiver bytes. Factory
attribute wrappers retain raw bytes; Factory list rewrites decline raw bytes or
comments they cannot preserve. Direct Rust tests cover these guards.

## Validation and remaining work

```sh
cargo test --locked --lib cop::rails
cargo test --locked --lib cop::performance
cargo test --locked --lib cop::factory_bot
python3 scripts/framework_parity/check.py \
  --ruby-project /path/to/wistia --output /tmp/framework-parity
```

Complete native suite: **898 passed, with 1 intentionally ignored audit probe**.
The strict command exits 1 with **418 explicit difference events**: 416 documented
safety/upstream-hazard events and two unresolved parser-related corrected-byte
differences. `failures.json`, `summary.json` and `runtime.json` retain every event.
Detection, message and range differences in the selected specimens are confined
to the documented guards and original corrector crashes; EnumHash's emoji symbol
spelling gap remains open.

This is finite evidence, not a proof over every Ruby program. Remaining branch
coverage includes unusual bindings, arbitrary AST/source equivalence,
version overrides, disabled Style/UnlessElse interactions, source scopes and
cross-cop interactions. AttributeDefinedStatically declines unsupported
multi-positional-value rewrites instead of generating invalid Ruby. Factory list
binding analysis remains conservative. No Wistia enforcement changes, rule
retirements, rollout changes or S3 releases are included.

## Fifth batch: Rails declarations and assertions

Reviewed the complete original rubocop-rails 2.26.0 implementations after
implementation: ScopeArgs receiver/arity/send versus block, local and safe-call
branches; AssertNot negation, delimiters and correction policy;
HasAndBelongsToMany receiverless selector forms; ActiveRecordOverride nearest
class, exact superclass source, four persistence names and forwarding versus
explicit-argument super. Added 47 original-rule differential specimens.
The matrix now covers 342 original specimens, 684 second-pass specimens and
20 trusted runtime programs, retaining exactly the same 121 explicit earlier
safety/upstream-hazard events, with no new differences. The Rails native suite
passes all 53 tests. AssertNot declines unsupported comment/newline/not
spellings rather than entering an original no-progress autocorrection loop;
these guarded syntax variants still require expanded differential evidence.

## Sixth batch: binding calls and mutable strings

Repaired two existing incomplete detectors, Performance/BindCall and
UnfreezeString. Post-implementation review compared their complete original
1.21.1 sources, including implicit `.()` calls, receiver/argument forms,
selector-to-call ranges, literal/interpolated strings, rooted constants,
parent-expression parentheses, unsafe metadata and version branches.
The reference driver now mirrors original RuboCop Team's minimum Ruby version
filter and places specimen version overrides under AllCops; direct Commissioner
invocation alone does not apply that filter. Seven version specimens run at
2.2, 2.3, 2.6, 2.7, 3.2, 3.3 and 3.4.

The expanded strict matrix covers 404 original cases, 808 second-pass cases and
23 trusted runtime programs. Performance native tests pass 34/34. There are
127 explicit discrepancy events, with six newly recorded events confined to
UnfreezeString with an attached block: original Ruby produces invalid
`+'' { work }`; Rust diagnoses but declines the correction. Encoding changes
from argumentless String.new remain explicitly unsafe, matching original
metadata; a trusted program records encoding before/after. No unexpected
differences remain in these selected specimens. Raw-byte/comment/heredoc
rewrites that cannot preserve source are conservatively declined; broader
coverage of those uncommon forms remains pending. Totals are 24 reviewed rules,
21 baseline stubs replaced and 46 baseline stubs remaining.

## Seventh batch: combined prefix/suffix checks

Performance/DoubleStartEndWith now implements matching send receivers/selectors,
second-argument purity, ActiveSupport aliases, original messages/ranges and
combined argument edits. Reviewed its complete original 1.21.1 algorithm and
rubocop-ast 1.32.3 Node#pure? branches after implementation. Structural comparisons
cover decoded string/symbol values, numeric bases, argument parentheses and
method calls. The existing Caller integer decoder is reused. RSpec has a similar
private structural helper; integration may consolidate these after both settle.

The matrix now includes 460 original cases, 920 second-pass cases and 25 trusted
runtime programs. Performance tests pass 36/36. There are 387 explicitly retained
difference events: the previous 127 plus 260 DoubleStartEndWith events. All 260
were independently checked to be receiver-preservation guards (only correctability
and corrected bytes differ), two original no-argument corrector crashes across
three passes, or the demonstrated runtime behavior difference. No new unexpected
detection, message, range or final-state differences remain in the authored cases.

Original on_or checks purity only for second-call arguments, then autocorrect
removes a second receiver evaluation. With a receiver method returning different
strings, original safe correction changes true/two calls into false/one call.
Rust reports the offense but declines to combine an impure receiver; ordinary
local and literal receivers still receive the correction. Original empty
argument lists crash when first/last.source_range is read; Rust skips them.
Comment/raw-byte deletions remain guarded. Exotic purity constructs, arbitrary
AST equality and all runtime contexts still require broader evidence. Totals:
25 reviewed rules, 22 baseline stubs replaced, 45 baseline stubs remaining.

## Eighth batch: enum duplicate values

Rails/EnumUniqueness implements modern/old keyword and explicit hash syntax,
array/hash value collection, duplicate grouping order and exact diagnostic
locations/messages/severity. Reviewed complete original 2.26.0 source, core
Duplication#consecutive_duplicates/grouped_duplicates and rubocop-ast Hash#values
after implementation. Hash splats are ignored by the original value collector.
The pre-existing stub's warning severity was corrected to original convention.
Shared Ruby expression normalization now supports this cop as well as Caller
and DoubleStartEndWith. The helper relocation is isolated in d07e3c0.

All 26 newly authored enum cases match exactly, including numeric spellings,
Unicode, escaped/quoted symbols and strings, nested collections, calls with
optional parentheses, percent arrays and multiline comments. Latest strict
matrix: 486 original cases, 972 second-pass cases, 25 trusted runtime programs;
exactly the earlier 387 deliberate safety/upstream-hazard events, no new
discrepancies. Complete native suite: 896 passed, 1 intentionally ignored probe.
EnumUniqueness is diagnostic only and never corrects source. Broader exotic AST
equality and actual production path-scope coverage remain outstanding.
Totals: 26 reviewed rules, 23 baseline stubs replaced, 44 remaining.

## Ninth batch: explicit enum hashes and Ruby literal formatting

Rails/EnumHash implements old/new declaration syntax, Rails >=7 positional
syntax gating, empty/percent arrays, canonical string dump/symbol inspect
formatting, interpolation, controls and character literals. Post-port review
compared the complete original 2.26.0 source and actual Ruby 3.4 dump/inspect
behavior. Dedicated shared/ruby_literal.rs distinguishes Unicode escaping in
String#dump from Symbol#inspect, escapes interpolations/control bytes and
quotes !@/~@ to prevent Ruby operator normalization from changing symbol values.
Lowercase/uppercase percent-array escaping and delimiters have authored cases.
The reference driver now applies TargetRailsVersion under AllCops; four versions
(6.1, 7.0, 7.2, 8.1) exercise the positional gate and old syntax.

Latest matrix: 527 original cases, 1054 second-pass cases, 27 trusted runtime
programs. Native full suite: 898 passed, 1 ignored probe. Two trusted enum programs
preserve the resulting mappings, including Unicode. Eighteen additional
documented safeguard events cover original comment deletion and an invalid
correction for `%W[a#{value}]` into `{a#{value} => 0}`.

One exact corrected-byte gap remains unresolved (two mode events): Ruby emits
`:😀` from Symbol#inspect; tree-sitter-ruby parses it as `(program (ERROR)
(identifier))`. Native keeps equivalent `:"😀"`, preserving the symbol value and
valid native syntax. This is recorded as a parser limitation, not claimed as
completed exact parity. Broader literal encodings/exotic ASTs and actual
production path scopes still need evidence. Totals: 27 reviewed implementations,
24 baseline stubs replaced, 43 baseline stubs remaining.

## Shared equality and integer follow-up

Added nine original-rule enum specimens for label/rocket and quoted-symbol
keys, implicit/explicit keyword hashes, trailing commas, brace/do blocks and
negative controls for regex flags/range exclusivity. All match original Ruby.
The shared signature now normalizes these specific forms while retaining
semantic operators; regex equality remains conservative raw-source comparison.
It is still a bounded key, not a complete Ruby AST adapter (for example,
operator-method and binary-expression spellings need broader normalization).

Imported core agent's shared integer decoder and replaced the duplicate decoder
with an alias. A scoped fix reads a signed literal's operand/operator directly,
supporting spaced signs without folding arbitrary unary expressions or nested
negations. Three enum negatives and a positive signed Caller offset match
original rules. A trusted negative-offset Caller program confirms why the
existing guard is needed: original caller(-1).first raises ArgumentError, but
Ruby's corrected caller(-1..-1).first returns a frame, changing behavior. Rust
preserves the original exception. These add 11 explicit safeguard events.

Latest matrix: 541 originals, 1082 second-pass cases and 28 trusted runtime
programs, with 418 retained events (416 guards/upstream hazards, two existing
emoji spelling gaps). Full native suite: 898 passed, 1 ignored probe. No new
rule implementations in this follow-up; inventory remains 27 reviewed,
24 baseline stubs replaced, 43 remaining.
