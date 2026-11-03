# Core rule efficiency changes

Base: integrated `2db3f585334e669e5b1466ded9ffadb1e7dff0ec` (implementation checkpoint `362240f`). This branch changes work performed by existing rules; it does not change supported rules, configuration, or correction guards. No production runtime improvement is claimed before the combined benchmark.

## Audit coverage

`docs/evidence/core-efficiency-coverage.json` lists 447 registered rules across the core and extension departments. All received static cost-pattern/dispatch screening; the eight hotspots received manual control-flow review. This is not a claim of exhaustive line-by-line performance proof for every rule. Reconciliation against the binary's complete 674-name registry found five names the first source extractor missed: Naming/BlockForwarding, Style/ArgumentsForwarding, Style/ExplicitBlockArgument, Cops/NoContentIndexCallsInBackfills and Cops/NoDirectLanternConfigBuilderMutators. Those implementations were reviewed separately; no additional bounded change was selected for this patch. The custom receiver policy cannot safely be narrowed to a fixed selector list; Lantern already has one. Forwarding analyses have different scope/parameter semantics and should not be combined without a separate measured design.

## Findings addressed

1. **SpaceAroundOperators:** iterate neighbor indices lazily instead of allocating all prior/later indices before testing the first neighbor. Avoid assignment-line character vectors and repeated token conversions while preserving Unicode and lossy-decoding behavior.
2. **HashSyntax:** cache parent-level syntax facts by the actual relevant option values, including first operator/rocket and wrapper bounds. Cache shorthand aggregates/final-pair indices separately. Both main and shorthand policy avoid rescanning all siblings for every pair. Facts retain no borrowed AST nodes.
3. **VariableName:** compile configured allowed/forbidden patterns once per source and relevant option snapshot. Reused mutable configurations are compared by value, not pointer identity. Empty option lists avoid creating the cache; duplicate forbidden matches and invalid-pattern skipping remain intact.
4. **MultilineOperationIndentation / IndentationWidth:** use indexed raw line access, including forward scans starting at the actual line, instead of repeatedly splitting file prefixes. `SourceFile::line_bytes` retains carriage returns, invalid UTF-8, and trailing empty lines exactly like `lines()`.
5. **ClassLength / ModuleLength:** share cached significant-line prefix counts, with and without comments. Module range queries retain exclusions for nested ordinary classes/modules and count singleton-class bodies as before.
6. **Trailing commas:** inspect bytes first; heredoc analysis is unnecessary unless a real comma occurs after a newline. When needed, literal policies share a sorted heredoc-start index instead of repeatedly walking nested literal subtrees; argument-tail heredoc facts are memoized with their existing special call semantics. Existing first/last argument command-recovery and unparenthesized-tail safety walks remain unchanged; their ordering-sensitive recovery is not replaced with a generic subtree summary.
7. **SpaceBeforeFirstArg:** cache word-start column counts once and subtract the current line's contribution. Preserve the original all-other-lines policy, full-line comment exclusion, and raw-byte column behavior.
8. **Shared dispatch/source phases:** queued `8873ea0` is the bounded selector-dispatch improvement, coordinated with the RSpec implementation agent. This branch does not duplicate that commit. A shared dispatch-plan lifetime cache or fused source walk is explicitly deferred: the audit did not measure setup as a current bottleneck, and either design must preserve per-path config filtering, correction masks, phase ordering, and edit ownership. The recent operand-order config bug makes caching by an approximate configuration identity unacceptable. The thread-safe analysis cache remains synchronized; existing concurrent initialization contracts are retained.

## Validation

- Full native suite: 1,101 passed, 0 failed, 1 ignored (the final run is recorded with its elapsed time in the evidence JSON).
- 27 focused trailing-comma tests pass, including captured original Ruby diagnostic/byte fixtures and guarded final states.
- New regression controls cover raw indexed CRLF/trailing-newline/invalid-UTF-8 lines, current-line/comment alignment exclusion, significant-line counts, mutable config separation, duplicate forbidden-pattern diagnostics, and cache reuse.
- `scripts/parity_audit/check_core_efficiency.py BEFORE AFTER --output PATH` compares 34 authored program/config cases in off, `-a`, and `-A` modes: complete normalized JSON, exit codes, stderr, and actual corrected file bytes. This is a frozen-native differential check, not a new Ruby oracle. Native original-reference tests remain the Ruby evidence.
- Diagnostic arrays are canonicalized for this supplementary CLI check: the unchanged baseline already emits two distinct VariableName orders across eight identical process runs (`docs/evidence/core-baseline-diagnostic-order.json`). No diagnostic content or corrected bytes are ignored.
- The code-simplifier skill was applied to the modified code: shared line counts replace duplicate class/module loops, cache records own their facts, and the common empty-options path avoids cache overhead.

Root integration owns the optimized full 14,824-file paired check and uncached/warm benchmark. No tarballs, S3 uploads, or rollout changes are part of this branch. Autocorrect incompatibility scheduling remains deferred as previously requested.
