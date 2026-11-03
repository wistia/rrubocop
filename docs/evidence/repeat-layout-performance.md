# Repeated layout and association processing

Audit snapshot: `8d4780e`; source implementation: `b92be3a`, branch
`perf/wistia-repeat-layout`. Covers assigned findings R2, R3 and R4 from the
repeated file-processing review. No shared SourceFile/config/regexp/heredoc files,
queued parity work, enforcement or correction guards were changed.

- **R2, IndentationWidth:** next-code indentation is now a lazy FnOnce argument.
  The unchanged prefix/suffix test rejects ordinary lines before any lookahead.
  Comment/blank runs therefore no longer trigger triangular searches from every
  preceding comment. Eligible dot/safe-navigation block lines still use the
  exact original forward scanner, including its raw-byte whitespace, comment,
  CRLF and keyword treatment. No new reverse-pass lexical policy was introduced.
- **R3, BlockAlignment:** opener-line retrieval uses indexed `line_bytes` instead
  of splitting from byte zero with `lines().nth`. Original fallback and slice-end
  bounds remain. The raw-byte oracle covers same-line lambda arrows, arrows on
  other lines, CRLF/trailing empty lines, invalid UTF-8 and recovered braces.
- **R4, HasManyOrHasOneDependent:** the existing explicit dependent/through test
  runs before parent normalization/DFS; its special argument-count condition is
  unchanged. The original ActiveResource/readonly predicate is cached by the
  normalized parent's range, native kind and Node/Block view. The distinction
  that skips a call's own block for a Node view is retained; this is not a broad
  class-wide exemption. with_options, numbered-parameter and extension logic
  remains unchanged.

Four new focused tests pass, including lazy-closure invocation controls, raw-line
old/new equivalence, Node/Block cache separation and cached-vs-old descendant
summaries. The coherent source passes **1,117 native tests / one pre-existing
ignored**, including existing original-reference tests, and the binary builds.
Manual code-simplifier review kept the change to these bounded mechanisms.

The fresh matrix contains **306 initial specimens** (all 258 existing association
specimens plus 48 authored layout/config controls), **612 derived final states**,
and **2,754 off/safe/all mode results**. The unchanged audit executable built
from `8d4780e` and the optimized executable produce **byte-identical entire JSONL
files**, SHA256:

`48deb58ccacf8517d8b07af6098da5d8ec166a5ff7c0a74fc367b424b0c9eba9`

This includes diagnostic messages/ranges, correctability/correction flags,
corrected bytes and final-state behavior. All native second passes are stable;
there are no invalid native corrections in this matrix. The public CLI on the
74 previously flagged Wistia files also matches the frozen source-equivalent
`0a46f7d` baseline's entire JSON/exits/stderr: 74 inspected, the same three
original offenses, zero corrections, exit 1 and empty stderr.

The strict original Ruby checker still reports **162 differences**: 57 diagnostic
comparisons, 42 safe-byte comparisons, 42 unsafe-byte comparisons, and 21 original
cop errors. They are preserved, not allowed away or called a parity pass. The
exact baseline replay proves that these optimizations add zero differences.

Deterministic compressed inputs, identical shared native results, Ruby results
and all strict failures are committed in `repeat-layout-matrix/`, with complete
source identities and compressed/decompressed hashes in its manifest. Replay
uses the existing `custom_parity_audit_probe` with RRUBOCOP_AUDIT_INPUT and
RRUBOCOP_AUDIT_OUTPUT. The data is parsed/linted; no untrusted fixture is executed.
The matrix includes both Width values, all three block-alignment styles,
comment runs, tabs/form-feed, safe-navigation/dot blocks and CRLF; invalid byte
and recovered-token checks are separate raw helper tests.

No production timing or release was performed by this agent. Complexity
reductions are source-backed; the parent owns integrated cold/warm measurement.
