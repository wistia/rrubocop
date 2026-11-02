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
