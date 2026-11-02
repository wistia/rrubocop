# Initial batch parity audit

The first full audit selected 3,420 examples across all 98 added cops. Ruby compilation excluded 5 invalid originals and 34 invalid generated variants; 3,381 examples had valid comparison baselines. Every cop retained valid coverage.

The run reported **346 candidate mismatch events**, including 240 range/severity differences, 48 other diagnostic differences, 51 correction-output differences, and 7 native correction errors. These are events across modes and examples, not distinct confirmed bugs. Existing safeguards and upstream hazards are recorded separately.

A generated example was also confirmed with the actual Ruby/native CLIs under `-A`:

```ruby
require "\x61"
require "a"
```

Pinned Ruby removes the duplicate second require; the fork keeps both. The generated corpus also surfaces character-literal require/rescue cases and blocked rewrites involving argument comments and encoded class names. Those remain candidates for a follow-up fix/review; this change adds the audit rather than changing the cops.

Runtime coverage is **10 explicit scenarios across 8 cops**. Original/native/Ruby outcomes include descriptor streams, exception/exit status, and watched file bytes. Runtime results currently identify inherited upstream hazards separately from fork differences.

Validation: 23 audit tests and all 807 normal Rust tests pass. The batch probe is ignored in normal test runs and exercised explicitly by the script. Complexity/formatting checks pass; Clippy has the same 223 preexisting diagnostics.

See [usage and limitations](../custom-parity-audit.md) and the [saved summary/groups/example](custom-parity-audit-initial.json). The command writes exact reproductions and raw results into its output directory. The full audit intentionally exits 1 while these candidate discrepancies remain; an eight-example CSV control audit exits 0.
