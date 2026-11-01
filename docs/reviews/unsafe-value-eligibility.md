# Return-value and key-eligibility safeguards

Implementation: `3272c7efd45a24e583ef9fb30e7336a29de2d8e7`. Cache rules revision: 34.

## Correction policies

`Lint/UselessTimes` is now report-only, including under `-A`. Ruby returns the integer receiver from a normally completed `times` block. A method ending in `1.times { :value }` returns `1`; replacing that expression with `:value` changes its return. An unused statement avoids that particular problem, but removing a block can still alter local bindings and the runtime frame. The old correction machinery has been removed. See the [Times policy](times-return-value-safety.md).

`Style/MapCompactWithConditionalBlock` is also report-only. Replacing `[nil, 1].map { |x| x if true }.compact` with `select` retains nil. Other cases involving `filter_map`, false, reassigned parameters, and arbitrary receivers also prevent a general syntax-only equivalence proof.

`Style/HashExcept` leaves dynamic key expressions manual: moving `keys` from the per-entry predicate into `except(*keys)` changes how often it is evaluated. Corrections remain available for supported static atoms and literal membership collections. Interpolation, splats, complex percent words, and scalar array keys are manual. Static controls preserve complete literal syntax. Existing unsafe assumptions about receiver and comparison behavior still apply to retained corrections; this guard does not establish equivalence for arbitrary receiver types.

All three cops retain diagnostics. These policies supersede the earlier Times and MapCompact rewrite policies, with explicit exceptions recorded against pinned RuboCop 1.65.1 references.

## Verification

- All 788 Rust tests pass; there are 2,362 pinned references across the 98 added cops, plus 11 interpolation references.
- The 28 new targeted references pass 84 release CLI checks across Off, Safe, and All correction modes. Static HashExcept controls still correct in All mode; manual cases preserve source bytes.
- All 28 targeted Ruby runtime comparisons preserve output and exit status. The three original hazard examples also preserve their original output with the release binary.
- Changed Rust formatting and whitespace checks pass. ABC reports zero function/module complexity violations and zero never-used warnings; 94 used-once advisories remain.
- Clippy remains blocked by the same 223 baseline diagnostics, with no additions or removals.

A matched uncached scan of the fixed 14,762-file Wistia corpus took 19.932s before and 20.059s after, with identical 27 CLI offenses. This is one scan per binary, not a performance estimate. [Measurements](unsafe-value-eligibility-measurements.json) record wall/CPU time and peak RSS. The corpus was unchanged at `404394655f71e35560f1c24bf773af3de098c534`; the baseline binary was `ae777f66aa48b0308c1ef16a4463217362304b67`.
