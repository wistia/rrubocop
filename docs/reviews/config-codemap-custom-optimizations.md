# Configuration, CodeMap, and custom dispatch optimizations

Implementation: `080e86680faf2137791d23aaa8c5b9e85d8c0166`; baseline: `06ed6194217e1505cd0662f56c3b23a083c70ac9`.

All four approved candidates are implemented:

- Prepared effective configurations merge once per directory group and lazily prepare only requested cop options. Parallel files and correction iterations share preparation; active cops borrow prepared options. Existing root-versus-file configuration behavior is preserved, including Syntax probes and correction validation.
- CodeMap removes empty ranges and merges touching/overlapping intervals once, then uses binary-search membership with unchanged half-open boundaries.
- Parent-kind dispatch skips autoload callbacks for ordinary constant references without parent searches. Registry and raw diagnostic order remain unchanged for CLI and MCP callers.
- A conservative source marker skips Redis transaction indexing when neither `multi` nor `pipelined` can occur. Marker matches retain the existing structural classifier.

Cache rules revision advances to 29.

## Validation

- 721 Rust tests pass, including 2,054 pinned Ruby reference examples.
- 67 before/after inspection cases have identical complete offense arrays; all inputs compile in Ruby 3.4.9.
- 21 before/after correction comparisons match both offenses and corrected bytes across Off, Safe, and All modes, including nested configuration and heredocs.
- CodeMap checks 46,656 interval triples plus bytewise membership on real Ruby syntax. Parent dispatch tests cover ordering and subtree traversal; Redis tests verify negative paths avoid index construction.
- Changed-file formatting and diff checks pass. ABC reports zero function/module offenses (59 advisory used-once findings). Clippy has 228 preexisting diagnostics, down from 230, with no new diagnostic identities.

## Measurements

Same release profile and fixed Wistia corpus (`404394655f71e35560f1c24bf773af3de098c534`), sequential processes with warm filesystem. Full runs alternate before/after then after/before; sample and actual nested-config runs use three repetitions per binary. Other scopes use two.

`--cache false` skips reads but still writes: each binary/scope used an isolated cache directory, keeping full-run entries below the pruning threshold. Synthetic and actual nested-config subsets use an unavailable cache backend to isolate lint/configuration work. No production cache policy was changed.

| Scope | Before median (s) | After median (s) |
| --- | ---: | ---: |
| sample | 2.484255 | 1.739732 |
| full | 28.654018 | 19.900803 |
| actual-nested-config | 3.997937 | 0.968158 |
| config-1-rootpair | 0.034500 | 0.034455 |
| config-1-nestedpair | 0.038263 | 0.038563 |
| config-100-rootpair | 0.036003 | 0.036255 |
| config-100-nestedpair | 0.206458 | 0.039308 |
| config-300-rootpair | 0.038065 | 0.037058 |
| config-300-nestedpair | 0.565604 | 0.039963 |
| codemap-8000 | 0.108072 | 0.062184 |
| codemap-32000 | 0.927038 | 0.195889 |
| constant-refs-2000 | 0.047430 | 0.042179 |
| constant-refs-8000 | 0.145899 | 0.117071 |
| ordinary-set-1000 | 0.034460 | 0.034751 |
| ordinary-set-10000 | 0.216452 | 0.209768 |

The full corpus includes 14,762 files and 27 offenses in both binaries. Median elapsed time improves by 30.5%; aggregate user CPU falls from approximately 219s to 149s. The combined result does not isolate attribution to individual changes. Configuration reuse has the strongest targeted evidence; ordinary Redis-set timings show only a small improvement at the larger size and no gain at the smaller size.

The actual nested-config scope is the same 16 dev/spec files plus a root Gemfile sentinel. Synthetic configuration pairs always select the root file first, comparing identical source under root and nested configuration with 1, 100, or 300 selected cops. CodeMap stresses literal-array membership; constant-reference stresses ordinary scoped references; ordinary-set includes unrelated method bodies.

Complete offense arrays match for every paired benchmark scope. Small synthetic timings include process startup and should be treated as directional. These are local Rust before/after measurements, not a forecast of CI wall time.

Raw timing rows, CPU/RSS medians, binary hashes, and parity comparisons: [measurement evidence](config-codemap-custom-measurements.json).

A subsequent [correctness and correction-indexing pass](correctness-and-correction-indexes.md) fixes interpolation, directives, Unicode ownership, and custom-rule edge cases, with separate measurements.
