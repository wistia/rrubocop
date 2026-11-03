# Measured cop dispatch candidates

Baseline: root performance checkpoint 35fcdb8. Parent phase-two instrumentation
recorded 281 million AST callbacks and 776 aggregate seconds in node callbacks;
NonNilCheck contributed approximately 49 seconds, RSpec/Focus 41 seconds and
RSpec/PendingWithoutReason 20 seconds. These are aggregate instrumented costs,
not isolated wall-time speedup estimates. The parent separately measured cold
scan reductions from 390.66 seconds to 98.48 seconds after its earlier fixes;
this routing patch was absent from that timing.

The existing early eligibility guards already restrict Focus/Pending to the
union of canonical RSpec EXAMPLES and GROUPS. Their walker selectors now derive
from those tables directly. NonNilCheck's existing branches require call names
!=, !, or nil?, or bare identifier nil?; binary/unary callbacks stay registered.
No configuration-dependent candidate names, cop algorithms, correction metadata,
original reference data or safeguards change. The existing walker API performs
one method-name lookup per call and routes through its name index.

Validation passes: 1,088 native tests / one pre-existing ignored, including the
unchanged original RSpec detection/correction and safety replays, original
NonNilCheck specifications and guards. The new comparison runs 142 original /
guard sources with correction collection off/on and a second pass in both routes.
Diagnostics (including metadata), raw edits, corrected bytes and final states
are identical. All original reference fixtures remain unchanged.

Counted initial callbacks across both collection modes:

| Cop | Sources | Unfiltered | Routed |
| --- | ---: | ---: | ---: |
| RSpec/Focus | 15 | 420 | 390 |
| RSpec/PendingWithoutReason | 30 | 640 | 482 |
| Style/NonNilCheck | 97 | 620 | 352 |

The 1,000 ordinary-call synthetic control invokes each unfiltered cop at least
1,000 times and routes zero callbacks, with identical outputs. The reference
corpora concentrate on relevant DSL/operators, so their reduction percentages
are not estimates of production callback or wall-time savings.

The fresh explicit 74-file Wistia 310b8e42e91 CLI result equals the frozen
optimized 35fcdb8 binary's entire parsed JSON under the identical invocation:
74 inspected files, 170 offenses, zero corrected, exit 1, empty stderr. This
explicit-file invocation reproduces one existing additional RSpec/DescribeClass
finding at dev/spec/rrubocop_installer_spec.rb:6 compared with the parent's full
14,824-file/169-offense scan. Both binaries reproduce that scope distinction;
no finding is filtered or allowlisted. Routing runtime gains remain pending a
separate optimized full-codebase benchmark.

Additional candidates (static hypotheses, not measured speedup claims):

1. Layout/SpaceAroundOperators Alignment::adjacent and assignment_neighbor eagerly
   collect every prior/later line index before breaking at the nearest eligible
   neighbor. Lazy iterators preserve iteration order and remove an O(file-lines)
   temporary allocation per alignment query. Alignment itself is built once/file.
2. Style/HashSyntax computes the same sibling list and all_symbols predicate for
   each pair. A k-entry hash therefore repeats k-element scans k times. Per-hash
   cached style facts could avoid that O(k-squared) work, but need configuration
   keys and existing original edge-case proof.
3. TrailingComma literal comma() scans the full literal subtree for heredocs
   before examining the suffix for a comma. Nested literals repeat subtree walks;
   inspect the suffix first or share a source index while preserving delayed-body
   newline safeguards. No profile yet identifies its contribution.
4. Local identifier membership already builds its model/index once. Each query
   still locks AnalysisCache's typed HashMap and clones/downcasts Arc. A typed
   once-cell handle could reduce overhead if sampled, but this is not evidence
   of repeated model construction or significant lock contention.

Generic autocorrection incompatibility/scheduling and syntax recovery remain
outside this performance task.
