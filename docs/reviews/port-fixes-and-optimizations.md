# Port parity fixes and performance optimizations

Follow-up to the [independent parity review](final-five.md) and [98-rule performance audit](ported-rule-performance.md), based on implementation `832a13f` and review record `d8470e3`. Fixes and optimizations are implemented in `3f5fca0`.

## Parity fixes

All six confirmed discrepancy groups are fixed. Eval excludes command heredocs, recognizes character and magic filename literals, and safely normalizes pure single-expression parenthesized location arguments. Quoted string heredoc delimiters containing backticks remain eligible. Unknown/effectful location expressions retain their conservative refusals.

Regexp capture state now evaluates deferred heredoc interpolation at its argument position, including nested and multiple pending heredocs. Plain element assignment does not invent a `[]` match; compound assignment preserves its getter semantics. Assignment lists and for-loop targets are covered.

ShadowedArgument now receives recursively bound nested/rest parameters from the file model and excludes pattern binding candidates. Default expression identifiers are not declared as parameters.

The four diagnostic-only checks remain diagnostic-only. The new `final-parity-regressions.json` contains 30 pinned Ruby cases, exercised through the actual correction loop with autocorrection off, safe autocorrection and unsafe autocorrection (90 checks). The original 2,024 cases additionally run through the real engine in all three modes, checking locations, messages, correctability and first-pass edit output. That comparison uses explicit cop selection, unrestricted fixture paths, Ruby 3.4 and ignored disable directives to match the reference Commissioner's isolated-check semantics; dedicated CLI/config tests cover normal filtering behavior.

Refresh the new references from the Wistia bundle directory:

```sh
bundle exec ruby /path/to/rrubocop/scripts/refresh_ported_reference.rb .rubocop.yml /path/to/rrubocop/tests/fixtures/final-parity-regressions.json
```

## Implemented optimizations

| Area | Change | Semantic safeguards |
|---|---|---|
| Local identifiers / NoSetex | Exact per-file byte-position index, shared from the engine model or built once on demand | Preserves introduction/read/write membership and lexical scope; only active consumers seed the index |
| Call dispatch | Method-name gates for 48 finite-name registrations | Two semantic wildcard rules remain ungated; binary and bare-identifier callbacks stay active |
| Tree walks / canonical forms | Streaming preorder traversal and one canonical output buffer | Preserves root inclusion, comments, subtree boundaries and encoding |
| DuplicateRequire | One source-order index per file | Keeps canonical argument equality, method and Ruby-parent boundaries |
| Autoload / require paths | Per-source root, expected-name, directory and existence caches | Fresh source contexts refresh filesystem state; distinct root policies retained |
| Inflections | Reused parsed acronyms with exact initializer-content invalidation and normalized lookup names | Same-size edits and deleted/missing initializers refresh correctly |
| Regexps | Lazy per-node shared token/capture analysis; capture-only scan emits no tokens | Preserves flags, interpolation, scanner state and source ordering |
| Configured regexps | Compile AllowedPatterns once per source/configuration, with cheap empty/allowed-method exits | Exact pattern keys isolate configurations; invalid patterns retain prior behavior |
| Shadow references | Lazy scope/provenance/read-position index for written arguments | Preserves binding/super, nested scope and branch handling; no index for unwritten arguments |
| Autocorrection | All 52 correctable registrations use lazy edit payloads and explicit feasibility | Diagnostics retain correctability without rendering edits; disabled corrections and atomic edit groups retain prior behavior |

Source analysis caches use typed keys and once-only initialization. They belong to a source snapshot, which is recreated for each autocorrection iteration; filesystem negatives and byte offsets cannot leak into a later source snapshot. No new dependency or global permanent negative filesystem cache was added. Rules revision increases to 26 so old cached results are invalidated.

These changes cover the concrete optimization categories in the audit. They do not replace every bounded vector or string allocation with a new checker API; those broad micro-optimizations had no demonstrated bottleneck.

## Validation and measurements

- `cargo test --all-targets`: 672 passed, zero failures.
- 2,024 original reference cases plus 30 new pinned cases (2,054 total).
- Original references exercise first-pass diagnostics and edits through Off/Safe/All; new cases additionally exercise the iterative correction loop (90 checks).
- Ruby 3.4.9 compiles all 591 changed correction outputs; no already-invalid inputs needed exclusion.
- Changed Rust files pass formatting checks; `git diff --check` passes.
- `abcop --full`: zero function/module ABC offenses and no never-used warnings; 19 advisory used-once suggestions remain.
- Additional Clippy audit on the installed Rust 1.96 toolchain remains blocked by preexisting lint errors. A detached `d8470e3` comparison found 231 baseline diagnostics and 230 final diagnostics, with **zero new diagnostic identities**. The one absent baseline warning is in an unchanged GraphQL file and is not credited as an improvement. No lint suppressions or thresholds were added.

## Controlled before/after measurements

Same release profile and unchanged application checkout; result-cache reads disabled with `--cache false`. No builds or other agent scans ran during measurement. Full scans ran sequentially in ABBA order, twice per binary; fixed 632-file sample scopes ran three times each with alternating binary order. Filesystem caches were warm. Times are medians; this is a single-host comparison, not a confidence interval or a promised CI runtime.

| Scope | Before wall seconds | After wall seconds |
|---|---:|---:|

| Full suite, 14,762 files | 31.334 | 31.444 |
| Full suite, 632-file sample | 2.533 | 2.551 |
| Added 98 only, same sample | 1.226 | 1.236 |

The full-codebase median changed from **31.33s to 31.44s (+0.35%)**; sample differences are also below 1% wall time. **No aggregate speedup is demonstrated on the current corpus.** All 27 full-scan diagnostics are identical, including their JSON fields. Most of the identified costs depend on candidates absent from the clean corpus, and the complete scan remains dominated by existing/shared work. The important measured improvement is the confirmed NoSetex worst case.

A diagnostic-only temporary project contains a method with local `redis`, `ttl`, `value` parameters and N identical `redis.setex("a", ttl, value)` calls. No correction flags are passed; every call remains reported.

| NoSetex calls | Before seconds | After seconds |
|---:|---:|---:|
| 50 | 0.140 | 0.033 |
| 100 | 0.369 | 0.035 |
| 200 | 1.367 | 0.041 |
| 400 | 5.363 | 0.055 |

At 400 calls this improves **5.36s to 0.055s (~98× faster)**. Doubling inputs no longer quadruples runtime at these tested sizes. This is a pathological-input improvement, not an extrapolated saving for the real application.

[Committed raw timing records and provenance](port-optimization-measurements.json) include both binary hashes, source/corpus revisions, sample manifest hash, CPU times, repetitions, file/offense counts and before/after diagnostic-equality checks. Broad micro-allocation/API rewrites were not used to manufacture a speed claim. No application sources were changed or autocorrected during benchmarking.
