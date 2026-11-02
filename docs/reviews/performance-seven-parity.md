# Seven transferred Performance rules

This branch starts at framework checkpoint b4ffbb8 and owns CompareWithBlock,
Detect, RedundantBlockCall, RedundantMatch, RedundantMerge, RegexpMatch and
StringReplacement. It preserves the completed RSpec branch. The machine-readable
[report](performance-seven-parity.json) tracks every assigned rule and discrepancy.

## RedundantBlockCall

Original Ruby source/spec review is pinned to RuboCop 1.65.1, performance 1.21.1,
parser 3.3.5.0 and Ruby 3.4.9. All twenty upstream source specimens are retained,
with argument/keyword, assignment, nested method/block, safe call, setter,
comment, disabled-correction and syntax/data controls added. Source import is
non-executing Prism extraction; imported code is never run. Upstream MIT licensing
is retained alongside the snapshots.

The differential matrix contains 78 originals and 156 second-pass states. Offense
counts/messages/ranges/severity/correctability match throughout; compilation and
idempotence have no differences. Forty-six originals match all corrected bytes.
Thirty-two deliberate safeguards preserve source; their 192 byte differences and
eight runtime differences remain visible, and the unchanged strict driver exits
one. They are not an allowlist or an exact-parity success.

Trusted programs prove original differences: no passed block changes NoMethodError
into LocalJumpError; nested parameter shadowing changes the invoked callback;
yield bypasses an overridden Proc#call; and multiline argument rewriting changes
__LINE__. A fifth control shows ordinary provided-block correction preserving
both supplied-block and absent-block outcomes. All ten native runtime states
preserve the original program. Syntax checks additionally retain invalid broader
heredoc rewrites. Corrections require block presence and binding identity and
preserve observed overrides, comments and line payloads. External dynamic
metaprogramming has bounded evidence rather than a universal equivalence claim.

The cached local membership index now also retains each declaration byte. Existing
membership is unchanged and no second parser/model cache is introduced. A dedicated
shadowing test and native correction regression pass. Logical implicit keyword
hash spans reuse the RSpec argument-view algorithm in a shared helper; its node()
API returns no physical node for a virtual implicit hash. Integration can delegate
RSpec consumers after validating that API distinction.

The actual production CLI/config scope gate passes 91 controls, including this
rule's positive and vendor-excluded files. Shared harness dependency gating was
imported as c6baaeb (framework 3517783). Generic disabled-correction metadata is a
separate integration concern; this cop explicitly matches the original flag.

## RedundantMerge and shared expression properties

All 28 original RedundantMerge spec sources are imported, including modifier
loops and MaxKeyValuePairs inheritance. Fifty-eight originals and 116 second-pass
states match diagnostics, ranges, messages and correction metadata exactly. The
unchanged strict gate retains 111 unsafe-mode byte differences and two runtime
differences; no diagnostic, compilation or idempotence mismatch was observed.
Corrections currently require one primitive literal pair on a literal Hash
receiver. Trusted programs demonstrate why unknown receivers and multi-pair RHS
ordering require safeguards; a literal Hash control preserves runtime output.
Invalid/coercion-error configuration and broader Ruby Integer string spellings
remain explicit bounded-evidence branches.

The shared Ruby pure?/value_used? adapter is checked against 101 direct original
AST observations, covering operators, dynamic word arrays, delayed heredocs,
numbered locals and expression/statement/condition/rescue/ensure contexts. It
uses the canonical Parser pair_label role adapter (79c8aab), not a new local
scanner. Original underscore shorthand values are CONST even when the VM may read
a local; AST purity is never treated as a general semantic-safety proof.
DoubleStartEndWith delegates to this helper: all ten added purity probes match.
Its existing receiver/comment safeguards and two original zero-argument errors
remain visible in the 66-original/132-second-state strict matrix. Generic combined
cop scheduling remains deferred.

The full native library gate passes 929 tests with one ignored; the final shared
helper gate passes after importing the canonical shorthand adapter. Production
CLI/config scope passes 93 controls across 42 rules. The static upstream importer
now records unresolved host templates for the five pending rules explicitly;
those are unfinished evidence, not clean scans or implemented registrations.

## RedundantMatch

All fifteen directly extractable original source specimens plus manually ported
shared-example precedence branches, negative arity/block/safe-call controls,
versions, heredocs and disabled correction are tested. Fifty-three originals and
106 second-pass states match diagnostics/metadata exactly. The strict gate exposes
42 safe and 42 all-correction byte differences and four runtime differences;
there are no diagnostic, compilation or idempotence discrepancies. Trusted case
matching shows MatchData versus Integer changing branch selection; a custom
receiver shows match versus =~ calling different behavior. Native retains those
sources. A literal regexp control preserves global match captures. Comments and
invalid generated syntax also retain source. Unknown receiver corrections remain
conservative; this is documented safety divergence, not exact correction parity.

## CompareWithBlock and expanded upstream templates

All thirty original shared-example expansions are retained. Seventy-seven
originals and 154 second-pass states match diagnostics/metadata exactly; strict
correction comparison retains 129 safe-byte, 129 all-byte and two runtime
differences. A trusted Item#key counter proves that key extraction changes method
evaluation counts. Correction is bounded to literal data arrays and a primitive
index projection; a trusted literal-array control preserves output. Comparison
uses the shared original AST send projection and expression equality, including
explicit operator calls, differing numeric spellings, safe calls, nonliteral keys,
block shapes and configuration controls.

The nonexecuting Prism importer now expands closed literal loops and shared
examples, yielding 4,720 original sources with no unresolved templates/placeholders.
This includes pending-rule evidence and does not mark those rules implemented.
Detect's sibling Style/CollectionMethods settings require the explicit global
fixture/config adapter being developed next. The expanded RedundantMatch gate has
80 originals/160 second states, still no diagnostic differences, and additionally
retains two invalid top-level yield host specimens (six original_invalid states).
Valid method-scoped yield controls are retained for the next gate.

## Detect

The original algorithm and all 82 upstream template expansions are reviewed.
The final gate has 159 originals and 318 second-pass states, with exact diagnostics
and metadata. It retains 240 unsafe-mode byte differences and one trusted runtime
difference: an early-exit search changes predicate invocation counts. Corrections
require primitive literal Array data and a literal predicate. Numbered blocks
reuse the scoped model adapter (bef4c25); safe calls, lazy chains, first/last/index
forms, arities, empty bodies and disabled correction are tested. The fixture bridge
replays the original sibling PreferredMethods nil/detect/find configuration.

## StringReplacement

All 132 original upstream template expansions match diagnostics, corrected bytes,
final state and idempotence exactly before additional safety cases. The expanded
150-original/300-second-state gate has no diagnostic/compilation/idempotence
mismatches. It intentionally retains 18 safe and 18 all-correction byte differences
and four runtime differences. Trusted programs expose lost regexp captures and
removed visibly overridden Regexp constructors; the native source is preserved.
Ordinary literal-string transformations retain trusted output. Constructors,
regexp flags, static string/symbol AST distinctions, StringInterpreter escape
semantics, bang variants, comments, arities and disabled correction are reviewed.
Non-UTF8 shared literal families remain bounded evidence.

The completed numbered-model import preserves synthetic-read filtering and the
existing cached binding identity test once. Its six focused numbered/context
checks pass. RegexpMatch remains under its separate 4,386-original upstream gate;
its new source is not included in this completed six-rule checkpoint.
