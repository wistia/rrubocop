# Framework performance implementation

Baseline integrated snapshot: `2db3f58`. Source branch:
`perf/wistia-framework-cops`, implemented through `90fb677`. Source-equivalent
frozen baseline CLI: `/dev/shm/rrubocop-full-paired/rrubocop-362240f`.
No queued parity leaves, rule retirements, enforcement/configuration changes,
fixture exceptions or release uploads are included. This is source-complexity
work, not a measured production runtime claim; the parent owns the benchmark.

Registry reconciliation covers **121 registered names**: 65 Rails, 25 Performance,
26 GraphQL, four FactoryBot and Migration/DepartmentName. The original 120-rule
inventory already counted Rails/ReversibleMigrationMethodDefinition. The missing
674th aggregate registry row was Migration/DepartmentName. The companion coverage
JSON enumerates all 121 names and was checked against the exact registered list.
A rule without an audit finding is retained, not certified fast.

## Finding dispositions

| Finding | Implementation / remaining boundary |
| --- | --- |
| F1 RegexpMatch repeated traversals | `d03a666`: cache ordered candidate/observer events per source, actual traversal-root span, scope and Ruby-version identity. A suffix-minimum boundary index preserves the first qualifying DFS event, including modifier-condition order; observer offsets use binary search with the original inclusive boundary. Indirect-capture observation is cached once/file. The unchanged old query algorithm remains a test oracle; 9,420 queries agree. Native call routing retains all four existing selectors and leaves binary dispatch unchanged. |
| F2 full-file proposal parsing | **Intentionally deferred**, with all existing full-file syntax guards retained in check and correction modes. Different candidate ranges generally produce different complete-source proposals, so a cache keyed by whole proposal bytes would retain O(E*N) bytes without demonstrated duplicate work. Skipping validation in check-only mode changes observable correctability; fragment/batch parsing can lose heredoc, precedence, line-literal and interacting-edit safeguards. No measured duplicate-proposal data justifies that memory cost or a parser redesign. F3 separately removes repeated immutable scans around those parses. |
| F3 file safety facts | `02db772`: cache Presence CRLF facts; only scan line literals when an edit changes line count, then cache the original predicate. Cache UniqBeforePluck relation overrides, RelativeDateConstant global vetoes/per-name counts, and LinkToBlank/RequestReferer line sensitivity using each cop's existing predicate. Veto discovery still short-circuits; existing conditional line-count checks remain. |
| F4 repeated GraphQL class scans | `02db772`: build one per-class ordered duplicate-definition index. Statement positions, first/last occurrences, gaps and class boundaries remain explicit; lookup emits in original callback order. |
| F5 enum recursive equality | `8502c7c`: bucket normalized integer values, verify matches with existing strict eql_expression, and retain opaque groups plus first-seen group order. The opaque fallback deliberately preserves old recursive comparison. Generic fingerprints for strings/encodings/complex/dynamic AST families remain deferred: using the weaker syntax signature could merge or separate unequal/equal Ruby nodes, while no corpus profile establishes a need to take that risk. Distinct integer enum lookup becomes linear expected work; opaque-only worst-case remains quadratic and is not hidden. |
| F6 GraphQL ordering | `02db772`: compile configured patterns once per source/effective complete order list, retain invalid-pattern false behavior, first-match priority, the last everything-else marker and lexical ties. Cache per-name ranks; cache hits avoid cloning order-list keys. Shared argument/name decoding is cached separately, keyed by full call byte range rather than start alone, preserving chained-call ownership. |
| F7 schema work | `02db772`: reject ineligible/conditional uniqueness calls before schema loading. Cache discovery, including absence, per operation and starting directory. Keep schema parsing's existing snapshot, first-match table lookup and external-index order using immutable maps. Cache per-class table inference. Missing/malformed schema and original expression-index fallback remain unchanged. |
| F8 metadata / allocations / routing | `02db772`: cache GraphQL method definitions per class with first-definition/singleton-body order, lexical action-filter definitions per class, and name decoding; preserve output order while using sets for unused-argument/already-grouped-name membership. Twelve GraphQL call cops route through existing canonical field/argument/public_argument selectors (LegacyDSL retains define). General class-body vectors/ancestor annotations are not replaced by a new cross-cop AST framework: remaining class callbacks mostly build their vector once, and retaining native nodes across SourceFile cache lifetimes is unsafe; byte-range rehydration/locking overhead needs profiling before broadening. Migration legacy-name qualification now uses a once-built first-entry map instead of fallback line scans. |
| F9 FactoryBot traversals | `90fb677`: short-circuit descendant searches; summarize dynamic and safe-navigation argument facts in one compatible walk; retain legacy numbered-binding and evaluation guards; use set membership for parameter names. The safe-navigation result is consumed only when dynamic is false, so short-circuiting on a dynamic expression preserves the original branches. |

## Validation

The coherent source checkpoint passes **1,102 native tests / one pre-existing
ignored**, including the unchanged pinned-original RSpec/framework/core tests and
safeguards, and the binary builds with deny(warnings). Six new focused tests cover
ordered observer equivalence, strict enum grouping/order/opaque fallback, schema
negative-cache operation lifetime and duplicate-table/index order, GraphQL class
gaps/boundaries, and config isolation including invalid regexes and fallback ties.

Fresh actual-Wistia-config CLI on the 74 previously flagged files matches the
source-equivalent frozen baseline's **entire parsed JSON**, exits and stderr:
74 inspected, exactly the same three original offenses, zero corrections, exit 1,
empty stderr. No offense is filtered out of that equality comparison.

The fresh matrix is complete: **6,383 initial specimens, 12,766 derived final
source/config states, 57,447 native mode results and 66 trusted runtime scenarios**.
The strict original Ruby checker reports **11,100 differences**, including known
safety/correction differences, original-invalid inputs and original cop errors.
They are not filtered, allowlisted or called a parity pass.

To distinguish existing differences from optimization regressions, the unchanged
audit executable was built from frozen `362240f` (source/harness identical to
`2db3f58`) and replayed on the exact same 19,149 input rows in the same working
directory. The baseline and optimized **entire 57,447-row JSONL files are byte-
identical**, SHA256 `182e5caa2222141de08bcacbca32eec6a6cac6d2bfddfb81144c1d218bfc26db`.
There are zero before/after diagnostic/correctability/flag/range/message/edit-
source/final-state differences. Trusted runtime programs' corrected source bytes
are consequently unchanged as well. All native second passes are stable and
there are no invalid native corrections in this matrix.

Deterministic compressed raw inputs, shared identical native results, Ruby
reference results, all strict failures and trusted runtime observations are
committed under `framework-performance-matrix/`, with compressed/uncompressed
hashes and source identities in its manifest. The original runtime job was
paused for the parent's quiet benchmark; its expired wall-clock timeout was
handled by rerunning runtime/postprocessing against the completed, identical
raw input/reference/native outputs. No case or failure was removed.

No production benchmark was run by this agent. Expected asymptotic reductions
and cached query counts are not elapsed-time measurements. Parent integration
must preserve the exact safety metadata and source guards when cherry-picking.
