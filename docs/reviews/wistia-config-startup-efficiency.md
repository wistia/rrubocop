# Configuration startup efficiency: R6 and R8

Base: `8d4780e32735490b42378407b5419801a7b5b2b3` on isolated `perf/wistia-config-startup`. This implements both assigned findings from the fresh repeated-file-processing audit. Neither finding is a claim of per-linted-file lockfile I/O, nor a measured production hotspot. The parent integration owns the full same-target benchmark and Ruby output reconciliation.

## R6: share lockfile bytes during one configuration load

`load_config` and `load_default_config` establish a synchronous, thread-scoped lockfile snapshot. Embedded-gem version selection and Rails/Rack metadata use it to read each requested lockfile path at most once, including absent/unreadable/non-UTF8 results. Snapshot state is restored on return, config errors and unwinding. An independent load gets fresh state; nested independent loads temporarily get their own snapshot, then restore the outer snapshot. Different threads do not share it.

The snapshot stores raw UTF-8 bytes, not chosen gem versions. Both original parsers remain unchanged, as do the `Gemfile.lock` then `gems.locked` per-gem search order, version override validation, exact/alias/nearest/baseline selection, warning emission and explicit TargetRailsVersion handling. A primary file lacking a gem or a parseable version does not suppress the fallback file. Paths are keyed as supplied by each working directory; distinct directories cannot contaminate each other's selection. No environment settings, credentials, process-global filesystem cache, or mtime shortcut were added.

The scope is intentionally synchronous: current configuration loading does not await or distribute version resolution across worker threads. Direct public gem-path calls outside these two configuration entry points continue to read fresh bytes. The existing private-gem `bundle info` path/mtime cache is unchanged: it is a separate mechanism, not a repeated raw-lockfile read in this finding. No remaining assigned R6 implementation is deferred.

## R8: prepare exact global exclusion verification

Each CopFilterSet now contains precompiled individual exact glob matchers in addition to the existing broad GlobSet. Exact matchers retain `literal_separator(false)`, expanded zero-depth `**/` variants, invalid-glob skipping and raw-path/lossy-string fallback. The broad `literal_separator(true)` prefilter is retained and still intersects the exact verifier; deleting this intersection would change behavior. Ruby regexp extraction, flags/current handling, RegexSet failure behavior, path forms and directory stripping are unchanged.

Both discovery and lint filter construction prepare these matchers once per filter set. Matching candidate excluded paths no longer expands/builds/compiles the exact globs. Nested/inherited configs continue to build their own effective filter sets, with independent exclude-replacement policies. Raw resolved-state/cop-config matching APIs are unchanged; this finding concerned the prepared normal discovery/lint path, not charging all cop filters per path. No remaining assigned R8 implementation is deferred.

## Validation and practical limits

Machine-readable results are in [validation.json](config-startup-efficiency/validation.json). New focused tests cover byte/absence lifetime, distinct directory keys, first-file versus per-gem fallback, malformed/invalid-UTF8 reads, plugin/runtime-parser sharing, explicit Rails targets, independent public loads including after errors, nested loads, threads and panic cleanup. They mutate files between queries to demonstrate the intended snapshot lifetime rather than merely checking cache container contents.

Exclusion tests compare against the pre-change verification algorithm across 11 patterns and a combined set, 21 paths in three relative/absolute forms, zero-depth/multiple-globstar variants, invalid globs/regexes, regexp flag handling, non-UTF8 paths, and real inherited/nested configurations through both discovery and full cop-filter construction. Existing config/version/availability/safety and original cop detection/correction suites remain active.

Formatting and non-test compilation passed. The coherent full native suite passed **1,121 tests, zero failures, one ignored** (90.52 seconds), including **68 configuration tests** and all eight new targeted tests. No source correction, dependency change, release upload or production-wide scan is performed by this branch. Source-based removal of repeated construction is established; actual speedup remains an integrated benchmark question.
