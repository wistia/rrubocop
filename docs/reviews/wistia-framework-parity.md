# Framework cop parity implementation

The initial inventory covers 90 enabled Rails, Migration, Performance and FactoryBot cops, including 67 baseline no-op stubs. This branch replaces 53 stubs and reviews nine existing implementations. All 67 baseline stubs now have bounded implementations across the owned/delegated branches; this does not mean every parity gap is closed. Four retained pre-existing rules still need review, and documented safeguards/parser/integration limits remain explicit.

Fourteen delegated baseline implementations are complete on separate branches awaiting integration: seven Performance at `00c5d3f`, four schema Rails at `fea4911`, and Delegate/FilePath/HttpPositionalArguments at `01f5a5b`. The ten transferred existing reviews are complete at `90b2798` on `feat/wistia-parity-rails-review`. This branch retains 66 of the original 90 rules. Inventory distinguishes its fourteen physical baseline stubs whose implementations live elsewhere from genuinely unimplemented rules (now zero), completed delegated reviews and four retained pending reviews.

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

Complete native suite: **910 passed, with 1 intentionally ignored audit probe**.
The strict command exits 1 with **472 explicit difference events**: 467 documented
safety/upstream-hazard events, two unresolved parser-related corrected-byte
differences and three upstream stale-hash detection differences. `failures.json`, `summary.json` and `runtime.json` retain every event.
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

## Tenth batch: conditional blankness and skipped callback options

Rails/SafeNavigationWithBlank implements if/unless/ternary and modifier forms,
original whole-conditional ranges and exact dot-token correction. Its original
SafeAutoCorrect:false metadata is preserved: -a leaves source unchanged; -A
intentionally changes nil's blankness behavior, identically in both engines.
Parenthesized/negated conditions, arguments, loops and comments have authored
negative/positive cases. A trusted ActiveSupport program covers the nil branch.

Rails/IgnoredSkipActionFilterOption implements all four callback selectors,
receiver/last-options requirements, precedence of only/if/except, symbol-key
filtering, duplicate-key handling and original left-space/comma ranges.
Post-port review compared the complete original 2.26.0 cop and core RangeHelp
space/comma algorithms. Thirty-nine added specimens (including two trusted
programs) cover aliases, quoted/rocket keys, explicit/implicit hashes, multiline
options, lambda values, duplicate keys, correction policy and negative contexts.

Thirty-eight retained safeguard events are confined to IgnoredSkipActionFilterOption:
original deletion of a first explicit-hash option leaves invalid `{, ...}`;
removing an option can delete comments or eager expression side effects. Rust
reports those offenses but declines the correction. The trusted option factory
increments a counter and returns a valid predicate lambda; Ruby correction drops
the increment, Rust preserves it. All new differences were checked independently
to be those correctability/source/runtime guards, not detector/message/range gaps.

Latest strict evidence: 580 original cases, 1160 second-pass cases, 30 trusted
runtime programs; 456 events (454 documented guards/hazards, two existing emoji
spelling gaps). Full native suite passes 902 tests, 1 ignored probe; Rails subset
passes 61 tests. Remaining unusual syntax and actual production path scopes are
still open. Totals: 29 reviewed implementations, 26 baseline stubs replaced,
41 baseline stubs remain.

## Eleventh batch: table timestamps and Ruby send roles

Rails/CreateTableWithTimestamps implements id:false search, exact one-argument
block matching, timestamp/datetime searches, &:timestamps exemption and original
header-versus-block ranges, preserving the official migration scopes and generated
ActiveStorage exclusions. Post-implementation review compared the complete
original cop and ActiveRecordMigrationsHelper block matcher. Bare identifiers,
local bindings, definitions, block parameters/locals, numbered blocks, nested
options and safe navigation have authored original-rule cases.

This review also closed concrete send-role gaps across earlier ports. New small
ported wrappers expose existing symbol decoding and classify bare-call identifiers
without confusing definition/alias/undef names or locals. OutputSafety now accepts
only actual regular translation sends, so raw(def t; end), translation setters
and safe-navigation translations still warn; root-qualified I18n constants are
recognized structurally. A shared setter-role helper excludes regular-getter
patterns on shorthand assignment targets, including multiple assignment and for
targets, while preserving nested getter and operator-assignment positive cases.
Fourteen owned ports use this opt-in helper; there is no global walker suppression.
The existing send-end helper moved to shared code with its Performance API retained.

Latest isolated-rule matrix: 638 original cases, 1276 second-pass cases and
30 trusted runtime programs. Exactly the prior 456 retained events remain;
no new detector, message, range, corrected-byte or idempotence differences in
these added specimens. Full native suite: 904 passed, 1 ignored probe.

New scope_check.py exercises actual production configuration through both CLIs
using virtual stdin filenames, without creating Wistia files. All 30 reviewed
rules have firing positive controls; 61/62 path probes match on common diagnostic
fields (cop, message, severity, start line/column). Cop-specific model, service,
spec, test, controller, migration and generated-storage scopes match. One global
runner issue remains: --stdin db/schema.rb --force-exclusion ignores Wistia's
AllCops exclusion in native, while Ruby skips it. Without --force-exclusion both
correctly inspect that path. This is delegated to root/core; no cop exception
was added. Scope script intentionally remains failing until the runner is fixed.
The separate emoji corrected-byte gap remains open as previously recorded.
Totals: 30 reviewed implementations, 27 baseline stubs replaced, 40 remaining.

## Resolved: actual-config stdin exclusions

Isolated runner fix 5d6de07 now honors AllCops exclusions for --stdin only when
--force-exclusion is passed, matching original Ruby's explicit-path behavior.
It distinguishes a skipped file from an inspected clean file, reporting zero
inspected files on exclusion, and returns before correction/file writes. Three
native tests verify an existing excluded file is untouched, a no-force explicit
path is inspected and an allowed force-exclusion path is still inspected.

The expanded actual production-config scope matrix now passes all 65 probes
across all 30 reviewed rules, comparing common diagnostics and inspected-file
counts. Schema/vendor global exclusions and no-force positive controls are
included; every rule has a firing positive control. No files are created in
Wistia by these stdin probes. The earlier global stdin gap is closed without
adding a cop exception. Full native suite: 907 passed, 1 ignored audit probe.
The isolated-rule matrix remains 638 originals / 1276 second-pass / 30 trusted
runtime programs with its 456 retained safety/parser events; this runner-only
change does not alter those internal linter cases. Two emoji corrected-byte
events remain unresolved. Broader nested configs/exotic AST coverage and the
40 baseline stubs are still outstanding.

## Checkpoint: consolidated expression comparison

Reused RSpec's committed comparator (4a9b2d1) and existing Context literal/span
logic in shared/ruby_expression.rs. Public internal APIs distinguish same_expression
(AST::Node#==, repeated NodePattern bindings) from eql_expression (strict
AST::Node#eql?, before duplicate hash grouping), recursively through arguments,
containers and blocks. Complex 1i/1.0i are == but not eql?; rational representations
normalize exactly, with numeric subtype distinctions preserved for strict equality.

Typed static string parts preserve the legacy str/dstr boundary for character,
physical multiline, concatenated and delayed heredoc literals. The existing
ruby_static_string single-part guard remains unchanged. Delayed body spans are
resolved by the existing reader; different bodies cannot compare equal merely
because their opener tokens match. Exact identical dynamic bodies additionally
compare interpolation nodes; other dynamic-body formatting remains conservative.
Regex comparison preserves escaped metacharacters according to pinned parser
3.3.5.0 lexer-strings.rb REGEXP_META_CHARACTERS/extend_string_escaped, normalizes
flags and equivalent delimiters, and compares interpolation expressions.
Setter/index/operator send shapes, empty/omitted block components and keyword
not parentheses have original controls.

A committed independently generated fixture records 48 original AST pairs and
both equality results (96 assertions), including large/subnormal complex numeric
comparisons, UTF-8 string classes and delayed heredocs. The reference generator
loads actual Wistia configuration before parsing; it never executes specimen code.
Full native suite: 908 passed, 1 ignored probe. Isolated-rule matrix now covers
680 originals / 1360 second-pass / 30 trusted runtime programs. All prior 638
cases remain stable. There are 459 retained events: the earlier 456 plus three
diagnostic events for the following explicitly unresolved upstream hash drift.

The pinned AST::Node constructor precomputes @hash. Continued-heredoc processing
can later mutate a StrNode child, leaving eql? true/current child hashes equal
but precomputed node hashes different. Original EnumUniqueness group_by then
misses a duplicate; native strict equality identifies it. Evidence retains both
results, including the original grouped flag. This is not hidden by an exception,
nor claimed as exact detection parity. Symbol multiline/byte-encoding and other
unreviewed AST families remain open; this checkpoint is a reusable bounded
comparison adapter, not a claim to model every Ruby construct. No new cop counts:
30 reviewed, 27 baseline stubs replaced, 40 baseline stubs remaining.

## Twelfth batch: Rails-aware dates

Rails/Date implements core/root-qualified Date recognition, flexible/strict day
policies, ancestor-send chain handling, AllowToTime controls, string zone suffixes,
deprecated conversions, exact selector ranges and unsafe correction metadata.
Post-port review compared the complete original 2.26.0 algorithm and TimeZone's
accepted-method vocabulary, reused from the existing constant. All ten accepted
methods have flexible/strict controls. Explicit strict style still honors
AllowToTime:true, matching original code despite broader prose documentation.
Block-body send roles, unrelated namespaces, safe navigation, option arguments
and disabled correction policy have authored cases.

The new date cohort adds 111 original specimens, including two trusted
ActiveSupport programs. Thirteen retained safeguard events concern strict
Date.current only: original correction replaces Date with Time.zone but leaves
current, introducing NoMethodError (TimeZone has no current method). Rust warns
but declines that unsupported correction; the trusted UTC-zone program preserves
the original Date result. Other unsafe -A corrections match original behavior,
and -a leaves these unsafe transformations unchanged. Symbol receivers are not
misclassified as string receivers after typed-part helper expansion.

RSpec's isolated 95fc00e symbol-shape follow-up is included as f259f70: multiline
dsym versus escaped sym and quoted-label shapes now have six additional original
pairs. Shared direct AST fixture has 54 pairs/108 equality assertions. Latest
strict rule matrix: 791 originals, 1582 second-pass cases, 32 trusted programs;
472 retained events (467 guards/upstream correction hazards, two emoji spelling
events, three stale-node-hash detection differences). No unexpected new date
detection/message/range or corrected-byte differences remain in the selected
cases. Full native suite passes 910 tests, 1 ignored probe. Actual production
CLI scope/metadata matrix passes all 67 probes across 31 reviewed rules with
every positive control firing. Totals: 31 reviewed implementations, 28 baseline
stubs replaced, 39 baseline stubs remaining.

### Rails/UnknownEnv — pinned-source review

Ported rubocop-rails 2.26.0 `lib/rubocop/cop/rails/unknown_env.rb`, then
reviewed the original `spec/rubocop/cop/rails/unknown_env_spec.rb` against the
implementation. The implementation follows Ruby 3.4.9 DidYouMean
`spell_checker.rb`, `jaro_winkler.rb`, and `levenshtein.rb`: codepoint matching,
length-dependent thresholds, normalized candidate selection, original-spelling
sort scores, duplicate dictionary entries, reverse ranking, edit-distance
selection and the one-result fallback. The pinned runtime has DidYouMean; an
artificial Ruby runtime without that library is not emulated.

Coverage includes every original predicate/equality spec shape, both equality
orders, explicit operator sends, root-qualified/whitespace-qualified Rails,
unsupported namespace/safe-navigation/argument/block receivers, symbol and
interpolated/concatenated/multiline string negatives, character/heredoc strings,
case/Unicode/@ normalization, duplicate/custom/empty/null dictionaries, Rails
7.0 versus 7.1 `local`, and no mutation of the configured dictionary. Fixtures
explicitly supply Wistia's production dictionary where appropriate: the original
reference inherits it, while the isolated native probe supplies per-case options.
This prevents a test-only default-config mismatch from being mistaken for a
spelling algorithm difference. This diagnostic-only rule never corrects source.

The full 877-case original matrix has no new UnknownEnv discrepancy and retains
exactly the prior 472 documented events. The final focused 91-case/182-second-pass UnknownEnv gate, including block
receivers, has zero differences against original Ruby. Production CLI/config scope tests
pass all 69 probes across 32 reviewed rules, including an UnknownEnv positive
control. Native suite passes 911 tests, 1 ignored audit probe. Totals are now 32
reviewed implementations, 29 baseline stubs replaced, 38 baseline stubs remaining.

### Rails/EnvironmentComparison — original correction safeguards

Ported and post-reviewed rubocop-rails 2.26.0
`lib/rubocop/cop/rails/environment_comparison.rb` and every branch of its original
`spec/rubocop/cop/rails/environment_comparison_spec.rb`. Reuses the UnknownEnv
regular Rails.env receiver matcher; no duplicated constant/send decoder. Both
string/symbol operands and equality directions, `==`/`!=`, explicit operator
calls, root qualification, safe-navigation/namespace/args/block negatives, static
versus dynamic literal shapes, disabled correction and source ranges are covered.
Normal string corrections preserve the actual receiver spelling and original
negation. Predicate spelling is validated as one parsed method call, preventing
an environment name from injecting a different expression.

Original Ruby calls all these corrections safe, but its symbol conversion
changes `Rails.env == :production` from false to true in production. Original
also turns `Rails.env == 'empty'` from true into String#empty? false. The port
preserves detection/message/range and declines correction for symbols, existing
predicate-method collisions, invalid predicate names, comment deletion and
heredocs. Existing predicate collisions were reviewed against Ruby 3.4.9 and
ActiveSupport 8.1.3.1 StringInquirer public methods; ordinary configured names
are not special-cased. Rails 7.1+ EnvironmentInquirer#local? also returns true
for development/test, while equality to 'local' returns false, so this correction
is guarded only for those Rails versions. The guard list remains a bounded
pinned-runtime inventory; arbitrary application monkey patches are not modeled.

The focused original-Ruby gate has 88 cases, 176 second-pass states and four
trusted ActiveSupport programs. Detection/message/range parity holds throughout;
all four native runtime cases preserve original behavior in both modes. The
strict driver retains 546 safeguard events: 180 correction-metadata differences,
180 safe-mode byte differences, 180 all-mode byte differences and six runtime
differences. No failures are suppressed, and native corrected outputs compile
and are idempotent. Native suite passes 912 tests, 1 ignored audit probe. The
production CLI scope matrix now has 71 probes across 33 reviewed rules. Totals:
33 reviewed implementations, 30 baseline stubs replaced, 37 stubs remaining.

### Shared regexp source-version adapter

Imported core's isolated 1a29876 literal-regexp predicate (545 original
observations), existing escape-decoder wrapper, and capture-observability scan.
Added `shared::ruby_expression::static_regex_value_for_ruby(source, node,
ruby_version)` and kept decoding in the existing Context symbols module, reusing
its escape implementation. The pinned parser 3.3.5.0 lexer
`extend_string_escaped` preserves normal regexp escapes, removes escaped LF/CRLF,
retains standalone escaped CR, and decodes c/C/m/M escapes only for Ruby >=3.1.
The prior raw-text adapter missed this version branch. Shared AST equality uses
the pinned Ruby 3.4 interpretation; cop consumers can supply their configured
version. Invalid UTF-8 remains unsupported by the String-returning static helper.

The reproducible `regex_reference.rb` fixture captures 64 original parser
observations across Ruby targets 2.5, 3.0, 3.1 and 3.4, with control escapes,
ordinary escapes, literal-delimiter escapes, static/dynamic/options negatives
and LF/CRLF/CR distinctions. The helper and existing 54 AST equality pairs pass;
full native suite passes 914 tests, 1 ignored probe. Invalid UTF-8 meta-escape
sources rejected by Ruby's compiler are not valid-source equality fixtures.

The follow-up adapter gate adds empty continued-regexp text: original parser
omits the empty text child after removing the continuation, including before
interpolation. Both original equality modes group those forms with empty/textless
counterparts. The direct fixture now has 68 version observations; shared AST
fixture has 56 pairs/112 assertions. Both focused native helper tests pass.
The differential reference's ProcessedSource parser now also honors each case's
TargetRubyVersion (it previously changed cop config but always parsed as 3.4).
This is required to test the lexer branch rather than configuration alone.

### Performance/DeletePrefix and DeleteSuffix

Ported rubocop-performance 1.21.1 `delete_prefix.rb` / `delete_suffix.rb`, original
`RegexpMetacharacter`, and every branch of both original spec files. One shared
anchored-substitution implementation covers gsub/sub, bang methods, safe sends,
Ruby 2.4/2.5 minimum gate, absolute versus multiline anchors, SafeMultiline,
nonempty deterministic literal recognition, regexp options, replacement-string
shape, disabled correction, selector ranges, and original unsafe correction
policy. Uses the shared original literal predicate, version-aware regexp AST
adapter, existing string decoder and capture-state scan. No second regexp or
Ruby string decoder was introduced.

Util#to_string_literal requires String#inspect spelling, distinct from
String#dump and Symbol#inspect. Existing shared ruby_literal now models those
three quote styles explicitly: ASCII controls use Unicode escapes for string
inspect but hex escapes for dump/symbol inspect; line/paragraph separators are
escaped. The bare-symbol candidate guard rejects ASCII controls and line/paragraph
separators even when tree-sitter incorrectly accepts them. Nine exact original
Util spelling observations and dump/symbol differences are native regressions;
44 EnumHash cases/88 second passes (two trusted programs) retain only its prior
20 documented differences, including the open emoji spelling gap. New control
and separator cases match original corrected bytes.

Native preserves original detection/message/range but declines unsafe original
corrections when regexp state is observed, comments/heredocs would be erased,
old-parser control/meta escapes are misinterpreted by StringInterpreter,
multiline anchors would change matching, or the receiver is a known Pathname
factory lacking these methods. Arbitrary application types remain governed by
the original unsafe-cop contract; no general type inference is claimed. Core
owns further structural Regexp.last_match receiver and raw CRLF adapter review.

Focused original gate: 258 cases, 516 second-pass states, eight trusted programs.
All native runtime programs preserve original behavior. The strict driver
retains 114 safeguard events: 54 metadata, 54 all-mode byte and six runtime
changes; safe-mode bytes match. The complete 1230-case/2460-second-pass matrix
(44 trusted programs) retains exactly 1132 prior/explicit events: 472 historical,
546 EnvironmentComparison guards and 114 DeletePrefix/Suffix guards. No new
unexplained detection/message/range differences remain. Native suite passes
916 tests, 1 ignored probe; all 75 production CLI scope probes across 35 reviewed
rules pass with positive controls firing. Totals: 35 reviewed implementations,
32 baseline stubs replaced, 35 baseline stubs remaining.

### Performance/StartWith and EndWith

Ported and post-reviewed rubocop-performance 1.21.1 `start_with.rb`, `end_with.rb`,
and both complete original specs, including every listed escape category, three
match methods, both operand directions, safe-send asymmetry, absolute/multiline
anchors, SafeMultiline and unsupported receiver/argument shapes. The four anchored
rules share one original regexp-literal reader and existing literal formatter;
DeletePrefix/Suffix's 258 cases retain their prior 114 differences after factoring.

Imported core 6797c88's grouped physical-regexp str adapter, raw CRLF normalization
and structural Regexp.last_match receiver guard. Original one-str matchers must
reject physical multiline regexps with multiple str children, rather than joining
them into a new accepted shape. Original adapter fixture now has 76 version
observations and 60 AST equality pairs/120 assertions. Start/end first-line
highlights also honor SourceBuffer CRLF normalization. Integration must retain
core's used joined `ported::decode_regexp_content` wrapper: this isolated branch
only needs the grouped wrapper, so its unused joined wrapper was removed for
`deny(dead_code)`; the underlying Context joined decoder remains available.

Original safe-autocorrection metadata is false. Native additionally declines
correction when match/`=~` results are observed as values (MatchData/integer
versus boolean), captures are observed, a block could lose effects, line anchors
would change matching, known non-string/nil receivers would change behavior,
comments would be lost, or a replacement changes receiver precedence. Predicate
match? preserves its boolean contract and does not alter regexp state. Ordinary
match/`=~` can correct in explicit truthiness conditions (if/unless/while/until,
ternary, negation and boolean chains whose whole result is a condition). These
are bounded syntactic/context safeguards, not whole-program type/encoding
inference; the original unsafe String-method contract still applies.

EndWith's third matcher also accepts a no-argument regexp send and captures its
method Symbol as the supposed receiver. Its corrector calls Symbol#source and
crashes, even when AutoCorrect is disabled because Ruby still builds correctors.
Native follows the source-defined warning but leaves it uncorrectable. Both
normal/disabled cases and their second passes retain six explicit upstream-error
events; diagnostic equivalence cannot be claimed against a crashing original.

The combined anchored gate has 834 original cases, 1668 second-pass states and
19 trusted programs (11 new start/end programs). All 19 native programs preserve
original behavior. Non-crashing original detections/messages/ranges match. The
new start/end rules retain 1243 explicit events: 615 correction-metadata, 615
all-mode bytes, seven runtime and six upstream errors. Safe-mode bytes match.
The full 1807-case/3614-second-pass matrix (55 trusted programs) retains exactly
2375 prior/explicit events: previous 1132 plus these 1243. Native suite passes
917 tests, 1 ignored probe. All 79 production CLI scope probes across 37 reviewed
rules pass, with every positive control firing. Totals: 37 reviewed
implementations, 34 baseline stubs replaced, 33 baseline stubs remaining.

### Shared command unary-plus token roles

Validation review exposed `validates_presence_of +1.5`: original Ruby parses a
command call with a numeric literal argument, while tree-sitter presents binary
addition. Core independently confirmed an unsafe existing layout correction:
`puts +1.5` became `puts + 1.5`, changing command-call semantics; nested
`foo +1 +2` must only space the second plus. The new
`shared::command_unary_plus(source, node)` identifies the original lexer role
using the existing cached local/vcall model, explicit-argument/block boundaries,
receiver calls and exact whitespace/continuation spelling. It does not edit
source and does not claim a complete logical-call argument projection.

The original lexer, not guessed AST text, supplies 318 observations (106 source
forms across Ruby targets 2.5, 3.0 and 3.4). Every native binary-plus token is
checked against original tPLUS versus tUPLUS/tUNARY_NUM roles, including declared
locals/parameters, explicit parentheses, receiver/safe receiver/operator calls,
compound expression grouping, comments, LF/CRLF continuations and Unicode
positions. The reproducible command_reference.rb accepts this fixture directly;
regeneration is identical, and its Ruby lint check passes. The focused native
role test passes all observations. Existing scope-model fixes remain owned by
core; this helper reuses that model without duplicating lexical bindings.

The fixture explicitly records invalid original grammar too: e.g. `[foo +1]`
gets a unary token but is rejected by original Ruby/RuboCop, whereas tree-sitter
accepts it. Preserving its token role avoids inventing a binary-spacing edit;
complete syntax equivalence and logical argument grouping remain separate work.
Core owns the operator-spacing consumer, while Validation integration continues.
No additional cop is counted implemented by this helper checkpoint.

### Rails/Validation and command argument bounds

Ported and post-reviewed rubocop-rails 2.26.0 `validation.rb`, every original
validation spec, all 12 macro names (including actual size alias behavior), and
the pinned AST Node#literal? categories. Covers positional/parenthesized attributes,
arrays/frozen/percent arrays, implicit/explicit hashes and splats, argument/type
negatives, receiver/local-variable roles, interpolation, comments, disabled
correction and signed command grouping. Uses existing local/vcall scope helpers;
shared `command_unary_plus_call` now preserves original call bounds and argument
partitions through binary/range/ternary grouping and first command arguments.
The 234 original syntax-accepted/error-free-native source/target combinations
match original selector, receiver, call-source and argument-source bounds. Native
parser-error forms remain explicitly unsupported instead of inventing an AST.
For bare signed commands whose generated keyword arguments cannot be represented
by the native parser, detection is retained and correction is declined.

Original on_send returns from *inside* its corrector for unsupported trailing
arguments, before Base#add_offense appends the offense. Disabled lines bypass
that block and still register disabled offenses. Native reproduces this pre-filter
behavior, retaining ordinary no-offense negatives and correct redundant-disable
policy. Four actual CLI policy probes cover supported/unsupported/no-argument/
unrelated bodies. Wistia disables Lint/RedundantCopDisableDirective by default;
the auxiliary probes explicitly enable it in an inherited temporary test config
and use the full cop set, because original CLI forbids selecting that auditor
with --only. No production configuration or enforcement changed.

The old ActiveModel 8.1.3.1 `_merge_attributes` recursively flattens attributes;
generic `validates` does not. Original correction breaks valid nested arrays,
array attributes followed by options, and unknown/nested splats. Native requires
known scalar attributes or proved-flat literal splats, expands only supported
last arrays, and preserves uncertain/known unsafe cases as uncorrectable warnings.
It also guards empty/dynamic percent-array invalid output, erased comments,
frozen-call argument side effects and non-array frozen receivers. Original frozen
bare calls crash on nil#parent; frozen hashes crash on HashNode#percent_literal?.
These remain explicit upstream errors, including the unavailable corrected
runtime comparison for the frozen-hash program. The strict writer now records
that missing reference result and native preservation rather than aborting with
a KeyError; no difference is suppressed.

Focused gate: 207 original cases, 414 second-pass states, 11 trusted ActiveModel
programs, all native programs preserving behavior. Non-crashing detections,
messages and ranges match. There are 340 retained events: 105 metadata, 105 safe
bytes, 105 all bytes, 14 runtime, nine upstream errors and two unavailable runtime
references. Full regression: 2014 originals, 4028 second passes, 66 trusted programs;
2715 total prior/explicit events (previous 2375 plus these 340), with no unexplained
new difference. Native suite passes 919 tests, 1 ignored probe. All 81 production
scope probes across 38 reviewed rules and four auxiliary policy probes pass.
Totals: 38 reviewed implementations, 35 baseline stubs replaced, 32 stubs remain.

### Typed enum AST follow-up

Re-reviewed `rubocop-rails-2.26.0/lib/rubocop/cop/rails/enum_hash.rb` (`enum_name`, `source`) and `enum_uniqueness.rb` (`enum_name`). Their `sym`/`str` branches use decoded values; `dsym`/`dstr` branches preserve source. Both ports now use typed shared symbol/string APIs, retaining raw empty/multiline symbols and multiline/chained strings instead of flattening their values. Character literals still use the existing decoder. Runtime decoding is unchanged.

Added 23 positive/negative enum-name/member specimens. The focused matrix now has 147 original cases, 294 second passes and two trusted programs; all new cases match. Its 23 retained events are the previously documented dynamic percent-array/comment safeguards and upstream stale heredoc AST hash discrepancy. Shared tests cover 75 original AST pairs, 76 regexp observations and typed string/symbol controls (four tests pass). No additional baseline stub was replaced.

### ReflectionClassName source/spec parity review

Pinned source: `rubocop-rails-2.26.0/lib/rubocop/cop/rails/reflection_class_name.rb`; upstream `v2.26.0/spec/rubocop/cop/rails/reflection_class_name_spec.rb` was read separately after the first port. All upstream branches are represented: the three association macros; optional scope argument; string/interpolated-string/plain-symbol exemptions; receiverless send exclusion; `to_s` on nonconstant receivers; local variables assigned string/symbol versus constants; shorthand options; correction limited to unqualified constants and their no-argument `name`/`to_s` sends. Additional negatives cover receivers, association arity, scoped constants, dynamic symbols, duplicate keys, splats, nested/forward local assignments, block arguments, safe sends, non-symbol keys and disabled autocorrection.

The final focused gate contains 99 original cases (including three trusted ActiveRecord reflection programs), 198 second passes and **zero differences** in detections, locations/messages/metadata, safe/all bytes, final states or idempotence. Original `Safe: false` is respected: `-a` retains source, `-A` performs the bounded original corrections. No new semantic safeguard discrepancy was found in these trusted programs; dynamic class-name contracts remain the upstream documented unsafe assumption. The post-port review repaired shorthand-value detection and safe-mode classification before this checkpoint. Actual production CLI: 83 scope probes across 39 rules plus four auxiliary disabled-policy probes all pass. Native suite: 921 pass, one ignored. This replaces one baseline stub; totals are 36 replaced, three existing reviewed, 31 stubs remaining.

### PluralizationGrammar original-rule review

Read pinned `rubocop-rails-2.26.0/lib/rubocop/cop/rails/pluralization_grammar.rb` and upstream `v2.26.0/spec/rubocop/cop/rails/pluralization_grammar_spec.rb`. Ported all 16 singular/plural method pairs, original absolute-value-one classification, integer/float receiver restriction, full send highlight and selector-only edit. Shared original-AST integer and float decoders are reused unchanged; `__LINE__` follows the already reviewed Int branch. Ruby float display is covered at decimal/scientific boundaries, Infinity and negative zero.

The separate post-port source review added safe sends, setters, arguments/blocks, unsupported methods, parenthesized/rational/complex receivers, integer radix/underscores, disabled policy and spaced unary signs. The initial gate exposed `- 1.days`: native syntax wraps the call in unary minus while original Ruby folds minus into its numeric receiver. A bounded direct-parent/numeric-token projection restores the original value and range; nested minus and explicit-parentheses controls are separate fixtures.

Final gate: **355 original cases, 710 second passes, three trusted ActiveSupport numeric/time/byte programs; zero differences** in detection, ranges/messages/metadata, `-a`/`-A` bytes, final states, runtime behavior or idempotence. Upstream randomized numeric specs were replaced with deterministic negative/zero/fractional/integer cases across every method pair. No deliberate correction divergence was needed in this bounded review. Totals: 37 original stubs replaced, three existing rules reviewed, 30 original stubs remaining.

Pluralization checkpoint validation: full native suite 921 pass, one ignored; all 89 CLI probes pass (85 unchanged production-config scopes across 40 rules, four auxiliary disabled-policy probes).

### HttpStatus pinned dependency/source review

Reviewed `rubocop-rails-2.26.0/lib/rubocop/cop/rails/http_status.rb` and upstream `v2.26.0/spec/rubocop/cop/rails/http_status_spec.rb`, including both checker classes and NodePattern alternatives. Pinned Wistia Rack is **2.2.24**; its full symbol/code dictionary was captured via `bundle exec ruby -rrack/utils` in `tests/fixtures/framework-http-status-rack-2.2.24.json`. Newer installed Rack defaults differ for 306, 413, 422, 509 and 510, so this port explicitly uses the pinned dictionary. Broader Rack-version dictionary selection remains outside this bounded Wistia review.

Coverage includes every dictionary entry in both styles, integer/string/symbol values, all five send names, permitted assertion aliases, custom codes, optional arguments, explicit/implicit hashes, first-versus-second hash precedence, nested non-status hashes, receivers, setters, numeric string prefix/underscore rules, primitive AST shape, source radix and `__LINE__`, comments, scope and disabled autocorrection. The distinct post-port review repaired hash-selection precedence before the final gate.

Final gate: **601 original cases, 1,202 second passes, three trusted programs**. Mapped-code detection/count/range/message/metadata/correction and idempotence match. **83 retained events** are deliberate: 27 correctability differences, 27 safe-byte differences, 27 all-byte differences and two runtime differences. Original `SymbolicStyleChecker#custom_http_status_code?` exempts unknown integer codes but rewrites unknown *string* codes to `nil`; native retains the warning and declines that correction. A trusted program explicitly pins Rack 2.2.24 and ActionPack 8.1.3.1: `ActionDispatch::Response.new('550').status` returns 550; the Ruby-corrected `nil` returns 0. Native retains 550 in both modes. The other two programs verify ordinary symbolic/numeric conversions. No allowlist suppresses these events.

Checkpoint totals: 38 baseline stubs replaced, three existing rules reviewed; 29 original stubs remain globally in this inventory, including seven transferred Performance stubs and 22 retained Rails stubs. Full native suite: 921 pass, one ignored.

HttpStatus production scope gate: all 91 probes pass (87 unchanged production-config scopes across 41 rules plus four auxiliary policy probes).

### IndexBy and IndexWith source/spec parity review

Reviewed pinned `rubocop-rails-2.26.0/lib/rubocop/cop/rails/index_by.rb`, `index_with.rb` and `mixin/index_method.rb`, then both upstream v2.26.0 spec files after implementation. Shared implementation handles all four original shapes: `each_with_object({})`, block `to_h`, `map`/`collect` followed by `to_h`, and root-qualified/unqualified `Hash[map]`. Coverage includes safe sends, multiline/empty argument parentheses, an outer `to_h` block, exact block-argument arity, unchanged element/no-op exclusions, memo reads, alternate receivers, setters, constant scope, comments, disabled correction and Ruby 2.5/2.6 boundaries. Rails dependency tests model actual `requires_gem` behavior; see `wistia-framework-dependency-fixtures.md`.

Final bounded gate: **254 original cases, 508 second passes, 12 trusted ActiveSupport programs**. All native programs preserve original execution. **120 retained events** comprise 36 diagnostic correctability differences, 38 safe-byte differences, 38 all-byte differences and eight runtime differences. Warning locations/messages/counts match. The deliberate safeguards are:

- Preserve comments that original suffix removal deletes; comments outside edited ranges or copied with the expression still permit correction.
- Decline break/next/redo/retry expression rewrites. Trusted map programs prove the original correction changes a NoMethodError/TypeError into a returned value/hash. The recursive guard remains conservative about nested control-flow frames; precision there is a follow-up, not a claim of exhaustive safe correction parity.
- Validate the entire proposed source; a captured splat array element would become an invalid bare block expression under the original correction. Native keeps the diagnostic/source.
- Produce the valid equivalent for explicit `Hash.[](map...)`; original fixed-prefix deletion emits invalid `[](x.index_by ...` / `[](x.index_with ...`. This intentionally differs in corrected bytes.

Post-port review added dependency-context fixtures, selector/setter exclusions, structural root-qualified Hash recognition and comment-range preservation. Actual CLI/config gate: **95 probes pass** (91 unchanged production scopes across 43 rules plus four auxiliary policy probes). Full native suite: **921 pass, one ignored**. Full previous-rule regression: 3,308 originals, 6,616 second passes and 75 trusted programs retain exactly expected events; final expanded Index cases were checked separately. Totals: 40 original stubs replaced, three existing rules reviewed, 27 original stubs remain (20 retained Rails, seven delegated Performance).

### LexicallyScopedActionFilter original source/spec review

Reviewed pinned `rubocop-rails-2.26.0/lib/rubocop/cop/rails/lexically_scoped_action_filter.rb` and the separate upstream v2.26.0 spec file after implementation. All 13 filter sends, both option keys, strict single-option/one-callback shape, nearest class/module ancestry, direct instance-method definitions, scalar/array string/symbol names, percent arrays, single-step aliases and ordering are covered. Post-port cases include proc callbacks/concerns, conditional calls, nested classes, visibility wrappers, singleton definitions, alias overwrite/chaining, Symbol-versus-String alias identity, duplicate action names, dynamic/static string AST shapes, receiver/safe-send/setter exclusions and disabled policy. The original first direct `begin` selection (including parenthesized superclass expressions) is preserved; this detection-only cop does not change code.

Final gate: **183 original cases, 366 second passes**. Every non-error source matches detection/count/range/message/metadata and unchanged safe/all bytes/final state/idempotence. **Six explicit upstream-error events** remain: missing/dynamic `alias_method` arguments make Ruby call `.value` on nil/a send node. Native skips those unusable alias entries and continues reporting the filter, rather than reproducing the crash. Existing EnumHash percent-word decoding was moved unchanged to `shared::word_value`, with interpolation rejection centralized. Both Enum rules were independently rechecked (147 originals, 294 second passes, two trusted programs); their 23 prior events are unchanged. Combined gate: 330 originals/660 second passes, 29 visible events.

Actual CLI scope/policy gate: **99 probes pass** (95 production scopes across 44 rules plus four auxiliary policy probes), including controller/mailer inclusion, model exclusion and the inline disable directive. Native suite: **921 pass, one ignored**. Totals: 41 baseline stubs replaced, three existing rules reviewed, 26 original stubs remain globally (19 retained Rails, seven delegated Performance).

### RedundantReceiverInWithOptions bounded source review

Read pinned `rubocop-rails-2.26.0/lib/rubocop/cop/rails/redundant_receiver_in_with_options.rb` and its separate upstream v2.26.0 spec file. Ported explicit and Ruby2.7+ numbered blocks, all-send receiver matching, normal nested-block exclusion, nested numbered blocks, multiple receivers on a line, receiver/dot deletion and exact parameter-space removal. Source review added operators/dotless sends, pseudo literals, setters, native unary-command projection, other receivers, local-name collisions, typed shorthand labels, extra/default/rest/block/destructured parameters, disabled partial corrections and proposed-source call validation. Cached scoped identities and numbered metadata are shared; no `_1` lexical classifier is used.

Final gate: **102 original cases, 204 second passes, 15 trusted ActiveSupport programs**. Offense counts/messages/ranges match every non-crashing original. **All 15 native programs preserve original output/errors**. **515 visible events** remain: 138 correction-metadata events, 165 safe-byte events, 165 all-byte events, 26 runtime differences and 21 upstream errors. Nothing is allowlisted.

Confirmed unsafe original transforms remain warnings without native correction: changing block invocation to `instance_eval` changes self/instance-variable reads or writes; remaining parameter reads and partially disabled receiver deletions break bindings; setters or collisions can reparse as local assignments/reads; original AST Const shorthand values can hide actual VM local/method reads. Dotless sends and subsequent numbered-block correction passes can crash upstream. Proposed-source validation also requires every removed receiver to remain a receiverless method send, preserving method/keyword distinctions.

Original explicit/private-call dispatch can differ from receiverless dispatch on OptionMerger. The 73 private method names in `src/resources/option_merger_private_methods_ruby_3_4.txt` were captured directly from Ruby3.4.9 / ActiveSupport8.1.3.1 with `ActiveSupport::OptionMerger.private_instance_methods.map(&:to_s).sort`. Actual `puts` and `__method__` programs prove the guard; this is visibility metadata, not offense suppression. Unknown/default/rest/block/shadow parameter semantics remain conservatively uncorrected; broader visibility versions, monkeypatches, parser-error recovery and additional frame cases stay explicitly open.

Validation: **929 native tests pass, one ignored**. **All 102 actual CLI/config probes pass** (98 production scopes across 45 rules plus four auxiliary policy probes), including partial inline-disable detection. Shared shorthand roles additionally match 45 original cases/48 roles; binding identities cover shadowing/numbered/lambda/field distinctions. Totals: 42 baseline stubs replaced, three existing rules reviewed; 25 original stubs remain globally (18 retained Rails, seven delegated Performance).

### RedundantAllowNil source/spec review

Read pinned `rubocop-rails-2.26.0/lib/rubocop/cop/rails/redundant_allow_nil.rb` and its separate upstream v2.26.0 specs. Preserved the recursive first-complete-pair search, exact bare-label key matching, original AST type comparison, both messages and first/middle/last sibling deletion ranges. Typed values reuse existing literal/send/local/numbered helpers; the root send excludes its attached block while nested argument expressions remain searchable. Source review covers nested/different validators, literal/dynamic values, receivers, quoted/rocket keys, shorthand, duplicate keys, splats, comments and disabled policy.

Final gate: **187 original cases, 374 second passes, five trusted ActiveModel programs**. All warning counts/messages/ranges match; all native programs preserve original behavior. **269 retained events** are 87 correction-metadata, 87 safe-byte, 87 all-byte and eight runtime differences. Native declines dynamic flag evaluation, arbitrary nested callback data, duplicate/spread precedence and comment deletion. Trusted programs prove original changes validation/evaluation when dropping a dynamic true/false flag, removes callback data, or exposes an earlier duplicate/spread option. Standard literal flags and nested validator options correct exactly. Non-primitive correction/value-encoding shapes remain explicitly bounded follow-up work.

Native suite: **929 pass, one ignored**. Scope tooling now accepts an explicit `--cop` filter (default full catalog unchanged) for focused implementation checks; this rule's two production model/service probes are recorded separately. Previous 102 scope/policy cases are unchanged, bringing the catalog to 104 (100 production, four auxiliary) across 46 rules. Totals on this branch: 43 original stubs replaced, three existing rules reviewed; **17 retained Rails stubs remain**. The seven delegated Performance stubs are completed separately at `00c5d3f`; this inventory now marks that external implementation explicitly rather than treating them as still unimplemented.

### DynamicFindBy source/spec review

Read pinned `rubocop-rails-2.26.0/lib/rubocop/cop/rails/dynamic_find_by.rb`, `mixin/active_record_helper.rb`, and the separate v2.26.0 upstream spec file. The committed fixture preserves all 27 upstream examples and their original configuration. Ported receiver/class-ancestor eligibility, root-qualified ActiveRecord/ApplicationRecord shapes, exact raw-source receiver exclusions, AllowedMethods/AllowedReceivers/legacy Whitelist, bang finders, Ruby trailing-empty split behavior, argument counts, hash/splat exclusions, attached blocks and receiverless bare sends. Unsafe correction policy matches the original `Safe: false` classification.

Distinct review after implementation covered nested non-model classes under a model ancestor, explicit nil configuration, empty generated keywords, block-pass arguments, multiline/comment trivia, Unicode/uppercase names, duplicate columns, setters and actual ActiveRecord::DynamicMatchers behavior. Final broad matrix: **473 originals, 946 second passes, five trusted programs**. All valid-original warning counts/messages/ranges match, every native corrected source compiles, all second passes are stable, and all native programs preserve original stdout/exit. **118 visible events** remain: 57 correction-metadata events, 57 unsafe corrected-byte events, three original-invalid setter-input states and one trusted-runtime difference. No exception list hides these results.

The original can insert keyword labels before block passes or generate empty keyword names, yielding invalid Ruby. Native retains those warnings without correction. The original duplicate-column rewrite also changes the argument selected by ActiveRecord's generated method (`first` becomes `second`); native preserves duplicate-column inputs. Standard single/multiple-column, bang and safe-navigation matcher programs agree. The authored runtime programs also explicitly pin ActiveRecord 8.1.3.1 for a separate repeat gate; the broad gate exercised the installed default 8.1.4. Custom user-defined finder methods and schema-dependent attribute aliases remain bounded unsafe-contract follow-ups, with existing AllowedMethods/AllowedReceivers controls preserved.

Validation: **930 native tests pass, one ignored**; both production model/service CLI scope probes pass. The catalog now contains 106 probes across 47 rules (102 production and four auxiliary); only the new rule's two probes were rerun here. Shared property helper import additionally passes all 101 pinned original observations and removes the duplicate DoubleStartEndWith purity implementation. Totals: **44 baseline stubs replaced plus three existing reviewed on this branch; 16 retained Rails stubs remain**. Seven Performance stubs are completed separately on the delegated branch.

### InverseOf source/spec review

Read pinned `rubocop-rails-2.26.0/lib/rubocop/cop/rails/inverse_of.rb` and the separate v2.26.0 spec file. The committed source fixture includes all 24 upstream examples with IgnoreScopes and Rails-version configurations. Ported all three association macros, consumed first-name argument, direct Hash option flattening, nil/false distinctions, inverse exemptions, through/polymorphic exclusions, scope checks and the Rails 5.2 `as` boundary. Original ordinary-block versus numbered-block decisions reuse the scoped model API with the Ruby 2.7 gate; no lexical `_1` scan was added.

Distinct review covers receiver/csend exclusions, nested with_options contexts, explicit/missing/mismatched/default/rest/keyword-rest/block/destructured/shadow parameters, nested helper blocks, duplicate options, explicit/implicit hashes, string/symbol/quoted/shorthand keys, attached macro blocks and Ruby 2.5/2.7/3.4 scope branches. WithOptions context matching preserves the original name comparison, including its lack of binding-identity sensitivity. Only receiverless original with_options commands with ordinary blocks contribute inherited options.

Final gate: **703 original cases, 1,406 second passes, zero differences** in detection counts/messages/locations, metadata, safe/all bytes, compilation and idempotence. The original is detection only, so source remains unchanged in every mode. **930 native tests pass, one ignored**; the two new production model/service scope probes pass. Broader exotic receiver AST shapes and parser-recovery inputs remain bounded follow-ups. The scope catalog now contains 108 probes across 48 rules (104 production and four auxiliary); only this rule's two new probes were rerun here. Totals: **45 baseline stubs replaced plus three existing rules reviewed on this branch; 15 retained Rails stubs remain**. Seven additional baseline Performance stubs are complete on the delegated branch.

### HasManyOrHasOneDependent source/spec review

Read pinned `rubocop-rails-2.26.0/lib/rubocop/cop/rails/has_many_or_has_one_dependent.rb` and its separate v2.26.0 specs; the committed fixture contains all 35 original examples. Ported nil/false dependent/through values, first-keyword-splat literal extraction, first/last logical hash arguments, normal/csend/bare sends, immediate-parent ActiveResource/readonly exemptions and original with_options block matching. Preserved the original broad with_options receiver behavior: this cop does not use InverseOf's context-name comparison.

A private bounded Node/Block projection preserves original embedded-send parents, single-statement body collapsing, multi-statement Begin, explicit parentheses, extension blocks and recursive parent.parent stepping. Tests deliberately retain the original differences between a lone nested with_options block and one following a sibling statement. No general semantic-parent API is claimed. Distinct source review added numbered extension/with_options blocks at Ruby 2.5/2.7/3.4, parenthesized versus keyword-begin/conditional bodies, empty and split options, no-name bare calls, root-qualified/namespaced ActiveResource and normal/endless readonly methods.

Final gate: **258 original cases, 516 second passes**. Every non-crashing original matches exact warnings/metadata and safe/all bytes; native sources compile and second passes are stable. **21 retained upstream errors** are the seven empty-option inputs across their three states: original `extract_option_if_kwsplat` calls `kwsplat_type?` on nil, while native reports the missing option without crashing. The original is detection only, so all native sources remain unchanged. **930 native tests pass, one ignored**; both production model/service CLI scope probes pass. Broader exotic grammar/parser-recovery cases stay explicitly open.

The scope catalog contains 110 probes across 49 rules (106 production, four auxiliary); only this rule's two new probes were rerun here. Totals: **46 baseline stubs replaced plus three existing rules reviewed on this branch; 14 retained Rails stubs remain**. Seven additional Performance baseline stubs are complete separately on the delegated branch.

### RedundantForeignKey source/spec review

Read pinned `rubocop-rails-2.26.0/lib/rubocop/cop/rails/redundant_foreign_key.rb`, AST `parent_module_name`/`defined_module`/`const_name` helpers, BasicLiteral/Dstr value behavior, RangeHelp deletion methods and the separate v2.26.0 specs. The committed fixture expands all upstream examples into 32 cases for belongs_to/has_one/has_many/HABTM. Detection preserves strict receiver/argument/hash/type gates and the first matching foreign-key pair. The default key comes from association name, original `as` value, or original module context.

Source-backed context probes cover root Object, nested/root-qualified/dynamic namespaces, ordinary/numbered blocks, class_eval, global/namespaced Class/Module.new assignments, constructor arguments and singleton classes. Default inflections reuse the 285-row ActiveSupport property helper; numeric rendering reuses existing integer/rational decoders and the previously reviewed PluralizationGrammar float formatter. Dstr values use the separate 29-row original AST property helper, retaining interpolation source without evaluation. Original bool/nil/complex/regexp/other unsupported `as.value` errors are recorded; native does not invent corrections for them.

Distinct review added duplicate foreign/as keys, keyword spreads, explicit/implicit hashes, block/setter/csend exclusions, source comments, disabled policy, rational/float values and physical/chained/interpolated string shapes. Six explicitly pinned ActiveRecord 8.1.3.1 reflection programs prove standard belongs_to/has_many/as corrections and three original behavior changes: duplicate as options, a spread foreign key revealed by removal, and an interpolated as name whose AST source differs from its runtime value. Native preserves all six original outputs/exits in both modes.

Final gate: **480 original cases, 960 second passes, six trusted programs**. Exact warning counts/messages/ranges hold for every captured non-crashing original; native corrections compile and all second passes are stable. **441 visible events** remain: 87 correction-metadata events, 87 safe-byte events, 87 unsafe-byte events, 174 upstream errors and six runtime differences. Duplicates/spreads, interpolated as values, comment deletion and invalid proposed output retain their warnings without native correction; nothing is allowlisted. Dynamic heredoc/Xstr-valued as names, file pseudo-literal shapes and custom inflection/model-name behavior remain explicit bounded follow-ups.

Validation: **932 native tests pass, one ignored**, plus both production model/service CLI scope probes. The scope catalog now has 112 cases across 50 rules (108 production, four auxiliary); only the new rule's two probes were rerun here. Totals: **47 baseline stubs replaced plus three existing rules reviewed on this branch; 13 retained Rails stubs remain**. Seven Performance stubs are complete on the delegated branch; the 20 pre-existing owned implementations still marked pending in the inventory require their separate original-source review.

### ContentTag source/spec review

Read pinned `rubocop-rails-2.26.0/lib/rubocop/cop/rails/content_tag.rb`, Base#correct/add_offense, the separate v2.26.0 specs, and actual ActionView 8.1.3.1 legacy TagHelper/TagBuilder implementations. The committed fixture includes all 26 original examples with Rails 5.0/5.1 gem contexts. Ported strict receiver/count/value/name gates, Ruby line-anchored name matching, shared ActiveSupport underscore formatting, ordinary/projected unary commands and original nested-corrector bookkeeping. The original corrector block executes even in diagnostic/disabled-autocorrect mode, so eligible enabled ancestors suppress nested diagnostics; disabled lines do not. Original call/header and delayed-heredoc ownership facts are cached per source with no retained tree-node lifetimes or mutable cop state.

Shared raw argument grouping is the separately validated 71223ed implementation. The unchanged numeric/string value renderer and float formatter now live in a small Rails helper, replacing cross-cop dependencies. Combined regression records 480 foreign-key cases retaining their 441 known events and 355 pluralization cases with zero differences. Fixtures use helper paths because ContentTag excludes models/config; actual path enforcement remains separately tested using unchanged production configurations.

Actual ActionView programs prove original behavior changes: non-void conversions change HTML nesting and can execute previously ignored blocks; positional option hashes raise under keyword-only helpers; `escape` changes from an attribute to a control argument; nested tag serialization changes data attributes (including delayed heredocs); XML output becomes malformed; collapsed calls can change __LINE__ values. Native preserves those contexts and declines invalid proposed output or comment loss. Remaining supported default HTML void conversions are **unsafe -A only**, since raw serialization/XML contracts differ from legacy output; -a is deliberately demoted. The runtime programs observe parsed HTML/XML and event/value effects, not a claim that raw legacy/new HTML strings are byte-identical. Unknown custom tag APIs, XML aliases and broader exotic value forms remain outside this bounded default-ActionView contract.

Final gate: **418 originals, 836 second passes, 10 valid trusted ActionView programs**. All 20 native runtime observations preserve the original output/exit under those explicit observables. Every captured native-parser-supported valid original matches warning counts/messages/ranges. The existing `tag +1, options` tree-sitter ERROR shape retains **three explicit warning differences**, without private parser recovery. **1,373 visible events** comprise 415 correction-metadata guards, those three parser-warning events, 510 safe-byte events, 426 unsafe-byte events, 16 runtime differences and three original-invalid setter states. Native corrections compile and all second passes are stable; no allowlist suppresses discrepancies. Xstr/AST-object and dynamic-heredoc first-name values remain bounded shared-value follow-ups.

Validation: **933 native tests pass, one ignored**; both production helper/model path probes pass. The catalog contains 114 scope cases across 51 rules (110 production, four auxiliary); only this rule's new two probes were rerun. Totals: **48 baseline stubs replaced plus three existing rules reviewed on this branch**. The four schema rules BulkChangeTable, NotNullColumn, ReversibleMigration and UniqueValidationWithoutIndex are transferred to the separate schema agent/branch. **Eight retained Rails baseline stubs remain**, plus four delegated schema stubs and 20 pre-existing implementations awaiting their source review. Seven delegated Performance baseline stubs are already complete. Scheduling and publishing remain deferred.

### LinkToBlank source/spec review

Read pinned `rubocop-rails-2.26.0/lib/rubocop/cop/rails/link_to_blank.rb`, its separate v2.26.0 spec file, and ActionView 8.1.3.1 URL/Tag helper argument and attribute behavior. The committed fixture includes all 39 upstream examples. Ported all three macros, exact Str/Sym target/rel patterns, direct Hash children including Hash receivers, ordinary/projected command shapes, first blank target per hash and Ruby's six-byte default split behavior. ASCII/control and Unicode whitespace cases verify token boundaries; the existing byte decoder is exposed through a thin wrapper rather than duplicated.

Distinct post-port review checked original global-first-rel mutation and last-argument insertion against actual HTML-option positions. Native preserves existing symbolic rel tokens, appends inside quoted/percent literals without decoding/re-emitting escapes, and fixes block-pass HTML hash insertion. It declines ambiguous routing/receiver hashes, dynamic/omitted rel values, unknown keys, duplicate/spread precedence and unsupported heredoc edits. Source/target pair-level inline disables and disabled autocorrection are tested. Line-sensitive replacements consume the canonical integer pseudo-literal role; ContentTag's equivalent guard now uses that same API rather than send-like identifier membership.

Twelve pinned ActionView 8.1.3.1 programs observe link text, href/target/other attributes, existing rel tokens and evaluation events. The intended added `noopener` token is excluded from existing-token observations. All original inputs run successfully, and **all 24 native observations preserve that existing link data/evaluation**. Original fixes can drop `nofollow`, change URL query options, corrupt percent/escaped literal syntax or introduce previously escaped interpolation execution. Native applies semantically preserving token additions where supported rather than copying these defects. Dynamic-key and spread cases retain warnings when safe mutation cannot be established.

Final gate: **592 originals, 1,184 second passes, 12 trusted programs**. All captured valid native-parser-supported originals match warning counts/messages/ranges. The existing `link_to +1, target: '_blank'` tree-sitter ERROR shape retains **three warning differences**, with no private recovery. **935 visible events** are 261 correction-metadata guards, those three parser events, 327 safe-byte events, 327 unsafe-byte events, 14 runtime differences and three original invalid-UTF8 split errors. Native corrections compile and all second passes are stable. UTF-8/Unicode protocol cases use the strict LF-delimited JSONL loader; no discrepancy is allowlisted.

Validation: **933 native tests pass, one ignored**, three JSONL protocol tests pass, and both production helper/model scope probes pass. The catalog now has 116 cases across 52 rules (112 production, four auxiliary); only the new two probes were rerun here. Totals: **49 baseline stubs replaced plus three existing rules reviewed on this branch; seven retained baseline Rails stubs remain**. Four schema stubs are delegated and 20 pre-existing implementations await their separate source review. Seven delegated Performance baseline stubs are complete; scheduling/publishing remain deferred.

## Rails/RelativeDateConstant checkpoint

Reviewed the pinned `rubocop-rails-2.26.0/lib/rubocop/cop/rails/relative_date_constant.rb` and all 15 upstream spec examples after implementation. The fixture preserves those original inputs; the fresh pinned Ruby reference supplies messages, source ranges, metadata and correction bytes.

The post-implementation matrix contains 213 originals and 426 generated second-pass states, including all eight relative method names, nonmatching/argument/safe-navigation controls, postorder-first message selection, ordinary versus numbered blocks and Ruby-version controls, direct/qualified/conditional/multiple assignment, array RHS grouping, comment ownership, overlapping nested assignments, autocorrection-disabled and inline-disabled policy. On every valid noncrashing reference state, warning counts/messages/ranges/severity match. The native implementation rewrites 16 original inputs; unsafe-mode corrections are byte-identical to Ruby where permitted. Safe mode preserves bytes throughout. No native invalid correction or nonidempotence occurred.

All 541 strict difference events remain visible: 264 correction-metadata safeguards, 261 unsafe-mode byte safeguards, 10 original runtime behavior changes, three original clobbering errors for nested constant assignments and three original-invalid states for the reserved bare `until()` call. These are not passing exact-autocorrection parity claims. In particular, the original explicitly documents that constant consumers are not updated; it also defers arbitrary initializer effects/errors and can overwrite existing methods (`NEW` becomes `self.new`).

Native correction stays unsafe and only supports bounded passive ActiveSupport numeric/duration/date/time chains, standalone unqualified assignments, no visible constant consumers/reflection, no source method overrides or superclass, and no comments/heredocs/line-number effects. A pinned 197-name Ruby 3.4.9 Object/Class method table prevents overwriting inherited methods; regenerate it with `refresh_relative_date_methods.rb` (loads JSON, matching the capture environment). Unknown external constant consumers or monkey patches remain part of the unsafe method-contract assumption, not a guarantee of global behavior preservation. Nonmatching/unsupported contexts still receive the original warning with correction unavailable.

Twelve explicitly authored programs pin ActiveSupport 8.1.3.1 and all have valid, successful original executions (including rescued error observations). All 24 native mode observations preserve original stdout and exit status. They cover constant reads/reflection, side effects, line values, inherited constructor preservation, invalid date calls and supported rewrites. Five original unsafe corrections change these observations. The source-backed guards are intentional divergences, not rule exceptions.

The initial delayed-heredoc matcher limit is closed by the separately validated shared-helper followup below. Typed/dynamic date-expression branches outside the bounded correction model remain warnings without fixes. No claim of complete cross-cop scheduling or integration parity is made.

Validation: final differential artifacts `/dev/shm/framework-relative-date-reviewed-results`; 933 native tests pass, one audit probe remains intentionally ignored by default; both actual-production-config CLI path controls pass. Scope catalog now contains 114 production controls across 53 rules plus four auxiliary policy cases. Ruby generator passes pinned default-config autocorrection/lint, and regenerates the method table exactly. This Rust-fork helper is not a Wistia autoloaded application/spec file. Six retained baseline stubs remain: Delegate, FilePath, HttpPositionalArguments, Presence, SafeNavigation, UniqBeforePluck.

### RelativeDateConstant delayed-heredoc followup

Imported Core's canonical `heredoc_interpolations` implementation and original ownership golden from `29ee303`, preserving the production helper verbatim; the generator was simplified and linted without changing regenerated data. No Eval consumer was imported. The direct golden covers eight sources, nine bodies and eight interpolation groups with exact expression text and positions. The native helper test passes and the pinned Ruby generator reproduces the fixture exactly.

RelativeDateConstant now traverses those associated interpolation nodes through its existing postorder matcher. Six new original cases cover interpolated versus single-quoted heredocs, ordinary/numbered blocks, multiple delayed bodies and nonmatching `||=` RHS shape. The expanded matrix is 219 originals/438 generated states/12 valid runtime programs, with zero warning differences on noncrashing valid references and all 24 native runtime observations preserved. All 559 strict events remain visible: 273 metadata guards, 270 unsafe-byte guards, 10 original runtime changes, three original clobbering errors and three original-invalid reserved bare `until()` states. Heredoc corrections remain unavailable, preserving delayed-source ownership rather than imitating malformed method rewrites.

### Disabled correction metadata review

The distinct post-implementation review found that both new date/uniq ports correctly suppressed disabled edits but still advertised a supported edit as `correctable`. Both now explicitly honor `AutoCorrect: disabled` before assigning correction metadata. The final combined matrix has 587 originals/1,174 generated states and 17 valid runtime programs; all 34 native observations preserve originals. Date's current 556 strict events are 270 metadata guards, 270 unsafe-byte guards, 10 original runtime changes, three original clobbering errors and three original-invalid reserved bare `until()` states. The earlier invalid-state attribution to numbered Ruby-version probes was incorrect; inspecting the recorded source identified `until()` as the actual invalid input. Numbered-block version controls remain valid branch evidence.

## Rails/UniqBeforePluck checkpoint

Reviewed original `rubocop-rails-2.26.0/lib/rubocop/cop/rails/uniq_before_pluck.rb` and expanded its 13 distinct upstream shared-example/spec inputs through conservative/aggressive styles. Ported exact ordinary-send/constant receiver matching, ordinary-block parent exclusion versus numbered-block handling, selector-only ranges, missing-receiver aggressive behavior, and original hanging/trailing period whitespace geometry. Reused the canonical ordinary-send wrappers from isolated upstream-parity commit `832b08e`, imported as `5d55b34`; no duplicate AST name classifier was added.

Final original-based matrix: 368 originals/736 generated states, including both styles, namespaced/dynamic-scope/pseudo constants, locals, safe/namespace dispatch, inner/outer blocks, explicit/empty/forwarded arguments, parenthesized receivers, chained uniqueness, Unicode columns, comments/CRLF/line values, disabled policies and an explicit `SafeAutoCorrect` override. Counts/messages/ranges/severity match every valid original state. All 69 permitted native unsafe rewrites match original corrected bytes. There is no native invalid correction or nonidempotence. Default safe mode does not rewrite; the explicit caller override follows the original policy.

All 707 strict events remain visible: 345 correction-metadata guards, 345 unsafe-byte guards, 12 original-invalid states from four inputs, and five original runtime differences. Original `.uniq()`/arguments are left behind when only the selector is removed, potentially yielding invalid source or changing argument evaluation; numbered blocks can be moved to `pluck`. Native correction refuses those contexts, preserves comments/line-sensitive source and CRLF, validates proposed syntax, and guards source-visible `pluck`/`distinct`/`uniq` overrides plus dynamic method/class evaluation. These guards do not suppress warnings.

The original marks correction unsafe because SQL collation may differ from Ruby uniqueness. Native `-A` retains that explicit standard-relation/collation contract; it does not prove arbitrary database, association or outside-file monkey-patch equivalence. Five valid authored programs cover custom relation side effects, missing/extra argument behavior, block semantics and dynamic method overrides; all ten native mode observations preserve original stdout/exit. They exercise guarded contexts, not a real-database collation guarantee. The source-backed original changes are deliberately not reproduced.

Validation: `/dev/shm/framework-date-uniq-reviewed-results` contains the combined strict gate; 934 native tests pass, one audit probe remains ignored by default; actual production-config CLI controls pass 2/2 for this rule. Scope catalog now has 116 production controls across 54 rules plus four auxiliary cases. No scheduler or release work changed. Five retained baseline stubs remain: Delegate, FilePath, HttpPositionalArguments, Presence and SafeNavigation; 20 pre-existing rules still need their separate review. Inventory now records the four schema implementations reported/tested/pushed at `fea4911` as delegated complete, without importing their cop code.

### Numeric member-call starts and final ownership transfer

Added the bounded `shared::numeric_send_start` geometry adapter for SafeNavigation's forthcoming whole-send warning range. It changes only the beginning of an existing native member call whose receiver chain starts with an actual numeric token and has a structurally associated unary `+`/`-`, or the existing verified unary-command-plus projection. It does not recover parser errors, infer arbitrary send types, normalize argument ownership or claim a complete command-send source range.

The direct golden contains 114 original ProcessedSource observations across Ruby 2.5/3.0/3.4: integer/float/rational/complex and spaced/nested signs, receiver chains/blocks, commanded positive signs versus declared-local/explicit-call binary positives, parentheses, unary `~`/`not`, multiline continuations and Unicode positions. All selected call-start positions match original AST ranges. Original full source/end records are retained as evidence, while the helper test asserts only the start geometry it owns. In particular, `-__LINE__.probe` does not fold the sign into the original member-call receiver span despite the pseudo literal's final Int role; it remains an explicit negative control. The pinned Ruby generator lints cleanly and regenerates exactly.

Parent confirmed transfer of Delegate/FilePath/HttpPositionalArguments to the completed-schema agent on `feat/wistia-parity-rails-final` from `9e7b524`. Those three cop implementations remain untouched here. Retained baseline work is now only Presence and SafeNavigation; the inventory distinguishes these two from the three newly delegated pending rules and eleven already-completed delegated rules.

## Rails/SafeNavigation checkpoint

Reviewed the original Rails 2.26.0 SafeNavigation matcher/corrector and ActiveSupport 8.1.3.1 `core_ext/object/try.rb`: `try!` delegates through `public_send`, `try` first checks `respond_to?`, and NilClass ignores already-evaluated arguments. Expanded the upstream parametrized shared-example source inputs over ConvertTry false/true and TargetRuby 2.2/2.3/3.4 rather than interpreting registry presence as evidence. Reused canonical ordinary-send names, typed plain-symbol decoding, original Hash argument grouping and the separately validated numeric member-call start adapter.

Final matrix: 350 original inputs/700 generated second-pass states, with exact warning counts/messages/ranges/severity throughout. It covers unanchored ASCII-word dispatch matching, plain versus quoted/multiline/percent symbols and non-symbol negatives, missing/implicit/self/literal/nil receivers, setters, optional try conversion, safe/namespace dispatch, block forms, positional/keyword arguments, eager evaluation, comments/heredocs/CRLF, line-number roles and disabled correction policy. All 96 permitted rewrites exactly match Ruby's corrected bytes. Neither native syntax-invalid output nor nonidempotence occurred.

The 715 strict difference events remain fully visible: 231 correction-metadata guards, 231 safe-byte guards, 231 all-mode byte guards and 22 original runtime behavior changes. Original eager arguments/block conversion run even on nil, while safe navigation skips them; quoted symbol source slicing can produce invalid source; try's missing-method suppression is different; setter assignment can discard a method's return; implicit/self calls can change visibility behavior. Native correction guards those branches instead of reproducing them. Unknown `try` receivers remain warnings without fixes unless nil or a bounded literal/public-method case proves the dispatch contract. Setters require one argument, no block and an unused result. Unknown-nil bang receivers require passive arguments; non-nil literal receivers retain normal eager argument evaluation. Source-visible try/public_send/respond_to overrides, visibility declarations and dynamic class/method changes block correction. Standard core and ActiveSupport contracts outside the current file remain assumptions, not a global monkey-patch guarantee.

Twenty-four explicitly authored programs pin ActiveSupport 8.1.3.1; all originals execute successfully and all 48 native safe/all observations preserve stdout and exit. These include actual positive safe rewrites, ordinary/argument blocks, literal try conversion, numeric signs, unused setter results and actual line-role distinctions, together with guarded nil/string/hash/block-argument effects, missing methods, quoted symbols, custom dispatch, used setter returns and private self calls. Eleven original corrections change these observations. Source-sensitive facts are cached once per SourceFile. The line predicate is Core's unchanged tested `is_line_literal` implementation from `9a4b4ce`; actual __LINE__ versus same-named selectors/definitions is covered by original and runtime controls.

Validation: final strict artifacts `/dev/shm/framework-safe-nav-final-results`; 935 native tests pass, one audit probe remains intentionally ignored by default; actual production-config CLI controls pass 2/2. Catalog now contains 118 production controls across 55 rules plus four auxiliary cases. Cross-cop `Style/RedundantSelf` incompatibility/scheduling remains explicitly deferred. Retained baseline implementation work is now only Rails/Presence; three other final Rails rules remain delegated, and 20 pre-existing implementations still need separate review.

An additional evidence review is queued for ContentTag/IndexWith: the final-Rails agent demonstrated via actual HTTP CLI controls that pinned Team's explicit TargetRailsVersion gate also matters alongside Base requires_gem. They are isolating original-Team harness predicate reuse. Prior current-production-target scope evidence remains separate and preserved; fixture-only explicit-target evidence will be refreshed rather than silently treated as complete.

## Independent configured-Rails and locked-gem gates

The actual CLI controls from the final-Rails agent exposed a gap in the original fixture factory: RuboCop 1.65.1 Team checks explicit TargetRailsVersion in addition to Base's requires_gem predicate. Its marker lookup uses a different namespace from Rails 2.26.0's declared USES_REQUIRES_GEM_API marker, so the explicit-version check is active in this pinned combination. The isolated original-Team predicate call from `22ee3d2` is imported as `2212957`; the harness reuses that original implementation rather than cloning its algorithm.

A saved four-case before matrix at `/dev/shm/framework-explicit-rails-before-results` reproduced nine strict events. ContentTag now independently requires target >=5.1 and locked railties >=5.1; IndexWith requires both >=6.0. An inventory-wide declaration scan also found Pick already checked the explicit target but omitted the locked-gem gate; that gate is now implemented for its original >=6.0 minimum. IndexBy has no such original requirement and stays unchanged. Case defaults explicitly describe the actual reference project's railties 8.1.3.1 for the newly gated Pick fixtures; empty/old gem profiles remain independent negative controls rather than inheriting the default.

The focused matrix adds 90 target×gem controls across the three gated rules. Full four-rule regression is 770 originals/1,540 generated states/22 valid runtime programs. All 44 native runtime observations preserve originals. Only the previously recorded `tag +1, options` tree-sitter ERROR specimen differs in warning content (three states); no new warning differences were introduced. All 1,509 strict events remain visible: 457 diagnostics events (three parser states, 454 correction safeguards), 558 safe-byte safeguards, 467 all-byte safeguards, 24 original runtime changes and three original-invalid states. Earlier current-production-target evidence is preserved; earlier fixture-only explicit-target claims are superseded by this corrected gate review.

The real-file runner reuses the final-Rails agent's proven setup with a private temporary project, explicit target, physical Gemfile.lock, fresh Ruby/native CLI processes and the actual reference bundle. Thirty-six expected-positive/negative controls cover absent railties and versions 4.2.11.3/5.0.0/5.1.0/6.0.0/8.1.3.1 independently of low/high configured targets; all 36 counts match the original and expected outcomes. Eight current-Wistia-config path controls also pass across the four rules. Full native suite: 935 passed, one ignored. No release/scheduler work changed.

`framework-version-minima.json` records all fourteen explicit minimum-version declarations found in the ninety owned original source files, including the four minimum-Rails pre-existing rules still awaiting their delegated body review. This is a declaration inventory, not a claim that every pending rule is implemented or every inherited policy was exhaustively reviewed. Existing-review ownership transfer is explicit in the main inventory; no code from those ten rules was edited here.

## Rails/Presence bounded checkpoint

Reviewed the original Rails 2.26.0 Presence matcher/corrector and pinned ActiveSupport 8.1.3.1 blank/present/presence implementations after porting. Captured 42 original heredoc/template inputs, including all six relational-operator expansions, then added branch/matcher and raw-parent controls. The matcher ports positive/negative present?/blank?, if/unless/modifier branch inversion, structural repeated-receiver equality, missing/nil fallback, ignored begin/if/rescue/while branches and elsif policy. Original messages require raw Ruby left-sibling truthiness, including primitive method/variable names and missing class-super/rescue-variable slots; a native named-child-only scan is insufficient. Parentheses, method/class/module/block bodies, assignments, argument/return/yield/super slots, rescue/ensure and fallback command/arithmetic formatting are covered explicitly.

Final captured matrix is 187 originals/374 generated states, with all non-error native-parser-supported warning counts/messages/ranges/severity matching the original. The only warning discrepancy is original-valid `work +1, other` in the fallback: independent tree-sitter 0.23.1 inspection confirms ERROR, recorded in `docs/evidence/framework-presence-parser-limit.json`; three warning states differ. No private lossy recovery was added. Six original-invalid top-level-yield states remain visible. No complete universal Ruby parent/source projection is claimed beyond the captured branches.

Signed numeric receiver bindings use the tested numeric-send geometry plus a new scalar-sign equality wrapper around existing integer/float/rational/complex decoders; no duplicate literal decoder or changed general comparator algorithm was introduced. The added repeated-binding cases include spaced/signed literals, zero, equivalent rational/complex spellings and receiver chains. All 26 permitted unsafe native rewrites are byte-identical to original all-mode corrections. No native-invalid correction or nonidempotence occurred.

All 1,122 strict events remain visible: 348 diagnostics events (three parser states and 345 correction safeguards), 400 safe-byte differences, 348 all-byte differences, twenty original runtime changes and six original-invalid states. Native correction is deliberately unsafe-only. It declines effectful/unbound receiver reevaluation, source-visible presence/predicate overrides, formatting/precedence/void/loop hazards, comments/CRLF/line effects and invalid proposals. In particular, original fallback rendering can turn compact `<` into a method call, keyword `and` can evaluate an unintended fallback, and a trailing `until` can wrap the whole replacement. These warnings retain the original message but do not apply those behavior-changing rewrites. Stable variable/scalar contexts remain under the normal core/ActiveSupport contract; unseen external monkey patches are not proved equivalent.

Twenty authored programs pin ActiveSupport 8.1.3.1; every original executes successfully, and all forty native safe/all observations preserve original stdout/exit. They cover actual supported rewrites, blank/nil/false/zero values, negative predicates and unless, fallback arguments, changing/repeated getters, overridden presence/inconsistent predicates, captured binding mutation through empty?, compact comparison, keyword-and/until precedence and comments/line values. Ten original corrections change those observations. Strong supported-edit AutoCorrect-disabled and inline-disable controls are included rather than relying only on an already-guarded receiver.

Validation: `/dev/shm/framework-presence-checkpoint-results`; full native suite 935 passed/one ignored; current-production-config path controls pass 2/2. Scope catalog now contains 120 production controls across 56 rules plus four auxiliary policy cases. This closes the final baseline stub while preserving the correction safeguards and parser gap as outstanding parity limitations. No release/scheduler work changed. Remaining retained existing reviews are Migration/DepartmentName, Rails/DelegateAllowBlank, Exit, FindEach, RefuteMethods, RequestReferer, ReversibleMigrationMethodDefinition, SkipsModelValidations, TimeZone, and Performance/TimesMap.

The ten delegated existing reviews are verified pushed at `90b2798`; their reported 1,649 original/3,298 generated states and actual scope/lock controls are recorded in their handoff, not re-claimed as local integration tests. Their original-invalid Safe:false + SafeAutoCorrect:true CLI-validation discrepancy remains an integration followup rather than an allowlisted pass.

## Existing review: Performance/TimesMap

Reviewed the original Performance 1.21.1 TimesMap source and all 26 expanded upstream shared-example inputs. The prior implementation flagged calls without blocks, omitted the original csend/block/numblock shape restrictions, emitted a generic incorrect message/range and provided no correction. The port now matches the exact two original shapes: zero-argument times/map-or-collect with an attached ordinary/numbered block, or the one-block-pass send shape. Numeric literal versus dot-only nonliteral receiver handling and literal-specific conditional message suffixes match original AST roles. Whole-block/send warning ranges and header-only replacement preserve the attached body.

Final differential: 337 originals/674 generated states; zero warning count/message/range/severity differences, and all 75 permitted unsafe corrections match original bytes. Both aliases, dot/safe/namespace dispatch, literal/dynamic/pseudo/parenthesized counts, unsupported arguments/absent/inner blocks, explicit block passing, numblocks, comments/CRLF/line observers and supported-edit disabled controls are captured. Distinct post-implementation review found that do/end under a unary-plus command belongs to the outer command, not the native map node; the correction uses the existing original-backed command role projection and preserves curly-block/local-variable/parenthesized controls. No new shared helper or parser recovery was added.

All 775 strict events remain visible: 381 correction-metadata guards, 381 unsafe-byte safeguards and thirteen original all-mode runtime changes. Native unsafe correction requires an addressable nonnegative literal Integer count and a known non-nil block source, with standard Integer/Array contracts and no source-visible constructor/iterator/constant-shadow overrides. Dynamic and negative counts, floats/non-integers, oversized array lengths and unknown/nil block arguments stay warned without fixes. The original's &nil rewrite changes an Enumerator into an Array; negative/float/oversized counts change successful behavior or error types. Proposed source is syntax-checked, and comment/line/CRLF hazards remain guarded. Normal allocation/core contracts are assumptions, not a guarantee of arbitrary memory-pressure or external monkey-patch equivalence.

Twenty-one explicitly authored programs all have successful original executions (errors are intentionally rescued where observed). All forty-two native safe/all observations preserve original stdout and exit. They include actual positive block/block-pass/numblock/next/break rewrites, zero iterations, callback events, negative/dynamic/float/nil/string/array counts, lexical Array shadowing, iterator/constructor overrides, nil block passing, line values and a bounded oversized-count/early-break observation. Thirteen original unsafe corrections change those observations. No native-invalid correction or nonidempotence occurred.

Validation: `/dev/shm/framework-times-map-reviewed-results`; full native suite 935 passed/one ignored; current production-config path controls pass 2/2. Scope catalog now has 122 production controls across 57 rules plus four auxiliary cases. No scheduler/release work changed. This is the fourth locally reviewed pre-existing implementation; nine retained existing reviews remain, with all baseline stubs still implemented across branches.

## Existing review: RefuteMethods and RequestReferer

Compared both complete original Rails 2.26.0 sources/specs after implementation. RefuteMethods originally matched only implicit sends; its prior native implementation incorrectly matched explicit receivers, omitted bare method nodes and selector lengths, and did not honor disabled correction. Both fourteen-entry style directions now use canonical ordinary-send roles, excluding locals/declarations/aliases/setters and explicit self/other receivers. RequestReferer's prior textual receiver check incorrectly matched locals/safe navigation/argument-bearing getters, omitted request() shapes, and produced one-character/full-block ranges. The port now matches the exact implicit zero-argument request getter and zero-argument accessor, warning over the original send header while preserving attached blocks.

Final combined differential: 538 originals/1,076 generated states, zero warning count/message/range/severity differences. RefuteMethods contributes 391/782 and eighty permitted exact native rewrites. RequestReferer contributes 147/294 and twenty native rewrites: eleven byte-identical to original, nine deliberately preserve forced request() getter syntax. Original whole-send normalization drops those parentheses and can resolve an existing request local instead, changing dispatch. Source-visible custom methods, comments and actual line observers remain guarded. This is safer correction behavior, not a claim of exact all-byte parity.

All 437 strict events remain visible: 135 correction-metadata safeguards, 144 safe-byte differences, 144 all-byte differences and fourteen original runtime changes. The RefuteMethods subset is 123 metadata/123 safe/123 all/six runtime; RequestReferer is twelve metadata/21 safe/21 all/eight runtime. AS 8.1.3.1 TestCase aliases were inspected directly: plain assert_not/refute use different default failure messages, while the other mapped methods are aliases. Native plain-method correction requires an always-passing false/nil argument or a known non-nil explicit string message; bare no-argument names are not rewritten because preferred local-variable collisions can change parse roles. No enforcement rule or style option was removed.

Thirty authored programs pin ActiveSupport/ActionPack 8.1.3.1 and Minitest 6.0.6 as appropriate. Every original executes successfully, including rescued assertion failures; all sixty native safe/all observations preserve stdout/exit. They cover all fourteen assertion pairs, both plain style directions, custom/default failure messages and visible overrides, actual ActionDispatch accessor aliases, explicit/implicit request getters, local getter-name collisions and line values. Seven original corrections alter those observations. Standard Rails assertion/request alias contracts remain assumptions outside the current source; arbitrary external overrides are not proved equivalent. No native-invalid output or nonidempotence occurred in the captured matrix.

Validation: `/dev/shm/framework-assertion-request-reviewed-results`; full native suite 935 passed/one ignored; four current-production path controls pass. Scope catalog now contains 126 production controls across 59 rules plus four auxiliary cases. Seven retained existing reviews remain: Migration/DepartmentName, Rails/DelegateAllowBlank, Exit, FindEach, ReversibleMigrationMethodDefinition, SkipsModelValidations and TimeZone. Shared helpers and scheduler/release behavior were not changed in this batch.


## Existing review: DelegateAllowBlank

Pinned rubocop-rails 2.26.0 `lib/rubocop/cop/rails/delegate_allow_blank.rb` matches only implicit ordinary `delegate`, exactly two Ruby arguments, and a final Hash containing a plain Symbol `allow_blank` paired with literal true. The reviewed implementation uses canonical send/argument/symbol projections, matches the entire offending pair range and exact original message, and honors disabled correction policy. All three original spec cases were ported from tag v2.26.0; the expanded matrix covers receiver/safe-send roles, argument counts, explicit/implicit hashes, value types, key spellings, duplicate keys, blocks, and disable comments.

Distinct post-implementation review: **187 originals / 374 generated second-pass states**, zero comparable detection/message/range/severity differences. The strict harness retains **107 events**: 33 diagnostic correctability safeguards, 33 safe-mode byte differences, 33 all-mode byte differences and eight trusted-runtime differences. No allowances were added. Original replacement erases the Symbol/quoted spelling (`:allow_blank => true` becomes `allow_nil => true`), potentially calling a method; native declines that edit. Visible custom APIs and duplicate options also remain warnings without fixes. Ordinary label corrections match original bytes and preserve comments/line geometry. The shared helper algorithms were unchanged.

Nine trusted authored programs cover six custom delegation APIs and three actual ActiveSupport 8.1.3.1 declarations. Seven valid-original programs preserve behavior in all **14 native modes**. Two actual ActiveSupport programs instead demonstrate invalid `allow_blank` options raising ArgumentError: literal true is intentionally repaired to a valid declaration in both engines; false is not matched. These invalid-original repairs are separate from valid runtime preservation. Full native suite: **935 passed / one ignored**. Both actual Wistia production scope controls match (128 catalog controls, 60 reviewed rules; this invocation ran only the new two).

Local checkpoint now covers 53 replaced stubs and seven pre-existing reviews. Six retained reviews remain: Migration/DepartmentName, Rails/Exit, Rails/FindEach, Rails/ReversibleMigrationMethodDefinition, Rails/SkipsModelValidations and Rails/TimeZone. All baseline stubs remain implemented across the owned and delegated branches; integration/scheduling limits remain open.


## Existing review: Exit

Distinct original-source review used rubocop-rails 2.26.0 `lib/rubocop/cop/rails/exit.rb`, including its receiver-child destructuring and argument-count policies. **317 original cases / 634 second-pass states match exactly, with zero strict harness events**. The matrix ports all nine original spec sources and covers implicit bare identifiers versus locals/declarations, normal versus safe sends, qualified constants, receiver calls named Kernel/Process (which the original unusually accepts), parenthesized and block receiver distinctions, grouped keyword Hash arguments, block passes and disable comments. Both methods use the original constant message mentioning `exit`; highlight ranges select the actual exit/exit! selector, including explicit receivers and multiline calls.

The old fixture incorrectly expected the receiver range/message and omitted bare exit; those expectations now match the independent original observations. This rule never corrects, and both modes preserve every tested source byte, compilation state and final offense state. No fixture programs containing exit/exit! were executed. Full native suite: **935 passed / one ignored**. Both actual Wistia path-scope controls match. Catalog now has 130 production controls / 61 locally reviewed rules. Five retained existing reviews remain: Migration/DepartmentName, Rails/FindEach, Rails/ReversibleMigrationMethodDefinition, Rails/SkipsModelValidations and Rails/TimeZone. No shared helper algorithms changed.


## Existing review: ReversibleMigrationMethodDefinition

The existing implementation was independently reviewed against rubocop-rails 2.26.0 `reversible_migration_method_definition.rb` and `MigrationsHelper`; **no production change was necessary**. All 13 original spec sources were ported, including multiple migration directories. Expanded **225 originals / 450 second-pass states have zero strict differences**. Coverage includes unqualified/root-qualified class names versus nested names, exact ActiveRecord::Migration receiver nesting, float versus integer/multiple/signed versions, explicit [] calls versus safe calls/parenthesized superclass, descendant instance methods versus singleton/dynamic definitions, zero versus required/forwarding/block parameters, empty/comment-only parameter lists, nested classes/methods, conditional and private definitions, and the original whole-class highlight.

This noncorrecting rule preserves all tested source bytes, final offenses, compilation state and idempotence. The last full native suite remains 935 passed / one ignored; only Python cases and fixture/report data changed in this checkpoint. Two actual Wistia migration/model scope controls match. Catalog: 132 controls / 62 locally reviewed rules. Four retained existing reviews remain: Migration/DepartmentName, Rails/FindEach, Rails/SkipsModelValidations and Rails/TimeZone.
