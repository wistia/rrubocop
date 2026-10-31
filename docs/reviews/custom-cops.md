# Custom cop port review

Reviewed implementation commit `586bb4c` on October 31, 2026. Three independent
agents reviewed all 27 app-specific ports against Wistia's pinned Ruby cops and
ran adversarial probes through actual CLI correction paths. The parent agent
reviewed the shared correction engine. The original review commit recorded findings only. The following resolution
records the subsequent implementation fixes; Wistia configuration is unchanged.

| Review group | Rules | Correction exposure | Detailed report |
| --- | ---: | --- | --- |
| Migration and Redis correctors | 3 | Enabled under ordinary `-a` and `-A` | [Correctors](custom-cops/correctors.md) |
| Serializer policies | 5 | 2 correctors disabled by current Wistia config; 3 diagnostic-only | [Serializers](custom-cops/serializers.md) |
| Runtime, safety, policy, autoload, and AnyCable guards | 19 | Diagnostic-only; 19 positive controls under `-A` left files byte-identical | [Guardrails](custom-cops/guardrails.md) |

The clean application corpus and previous passing parity fixtures did not exercise
these adverse correction shapes. Matching an unsafe Ruby rewrite is not evidence
that the correction is safe. Findings below distinguish inherited behavior from
Rust-only defects and current exposure from disabled paths.

## Resolution on this branch

The fixes below supersede the original review recommendations and observations.
The original findings remain as a record of the adversarial inputs.

- Edits now carry an owning offense byte offset. Suppressing an offense suppresses
  all of its edits, and corrected-result accounting uses ownership rather than
  overlap with the diagnostic highlight. Corrected-only runs exit successfully;
remaining offenses still respect the configured severity threshold. Legacy corrections receive ownership at
  their callback boundary; edits with uncertain ownership are conservatively skipped.
- Conflicting offense groups are accepted or rejected atomically. Independent
  insertions at the same point run in separate passes, preserving valid output
  when both migration cops add options to an empty hash. Identical edits retain
  attribution for both diagnostics.
- Each candidate correction batch passes the native syntax gate before acceptance.
  A failure aborts without changing the original buffer or file. No Ruby runtime
  is needed for this defense; it uses the existing native parser allowances.
- Table options are inserted into the actual argument list or explicit hash.
  Existing shorthand and nil-valued keys are respected. Splats, unknown positional
  options, external heredoc arguments, and uncertain quoted/escaped option keys
  are left for manual review instead of overwriting settings.
- NoSetex correction requires exactly three arguments (plus an optional block
  pass), with stable literal or proven lexical-local TTL/value expressions.
  Effectful calls, bare method names, interpolated values, heredocs, splats, and
  uncertain hash arguments keep their diagnostic without a rewrite.
- Both serializer method-to-block rules are now strictly diagnostic-only. The
  unsafe correction implementations were removed, so even `AutoCorrect: true`
  cannot delete methods, lose bindings, change return/super behavior, or damage
  conditional declarations. This strengthens the existing disabled Wistia policy.
- All six diagnostic regressions were repaired. Nearby fixes also prevent payload
  hashes from masquerading as TTL options and preserve shorthand option checks.
  Protected writes through safe navigation are guarded even though the Ruby
  handler's send-only matcher misses them; this difference is explicit in fixtures.

The pinned reference snapshot retains Ruby's original results alongside explicit,
reasoned Rust safeguards. This preserves provenance without requiring Rust to copy
unsafe Ruby behavior. The Ruby originals themselves are unchanged. Their migration and NoSetex
autocorrectors retain the reviewed hazards; these fixes apply to the Rust fork.
The original Ruby serializer transformations remain disabled by Wistia policy.

Validation includes the full Rust suite, pinned Ruby reference cases, syntax
compilation of every changed reference output, real CLI suppression/count checks,
combined-cop corrections, and before/after runtime checks for Redis/table arguments
and ActiveModelSerializers field values. See
[validation metadata](../wistia-cop-validation.json) for final counts. The full
14,762-file corpus still reports exactly the same 27 known regexp findings.

## Shared-engine defects

### P1: Disable comments hide diagnostics but do not prevent rewriting

[`apply_directives`](../../src/linter/engine/mod.rs#L161) filters diagnostics through
[`filter_directives`](../../src/linter/engine/mod.rs#L223), but leaves their edits in
the correction buffer. The [correction loop](../../src/linter/engine/autocorrect.rs#L118)
still applies those edits. This is a Rust-only engine defect affecting the new
custom correctors; it is not behavior inherited from the Ruby originals.

Confirmed under actual `-a` with inline, block, `all`, multiline, and mixed-call
cases. For example:

```ruby
# Before
redis.setex(key, ttl, value) # rubocop:disable Cops/NoSetex
# Rust file after -a, despite the disabled offense
redis.set(key, value, ex: ttl) # rubocop:disable Cops/NoSetex
```

Pinned Ruby leaves the disabled call unchanged. With one disabled and one enabled
call, Rust rewrites both; Ruby rewrites only the enabled one. Config-level
`AutoCorrect: disabled` is a separate mechanism and correctly blocks the serializer
corrections under both modes.

Recommended fix: give every edit an owning offense identity, filter all of an
offense's edits when that offense is disabled, and apply grouped edits atomically.
Do not infer ownership from edit start lines: an offense can span multiple lines
and have edits outside its highlight. Regression tests must include an enabled
and a disabled offense in the same file and all disable/todo/enable forms.

### P2: A corrected NoSetex call can produce an empty successful report

[`reconcile_corrected`](../../src/correction.rs#L80) credits an edit only if it covers
the diagnostic's starting byte. NoSetex highlights the full call starting at the
receiver, while its first edit begins later at the selector. The edits are applied,
but the corrected diagnostic loses its credit and the loop drops it. The final
pass has no remaining offense, resulting in `offense_count: 0` and
`corrected_count: 0` even though the file changed. This also reproduces with the
current Wistia config, independently of disable comments.

Recommended fix: associate accepted edits with their owning diagnostic rather than
requiring byte-range overlap. Verify actual file changes, corrected counts, and
reported offenses together in integration tests.

## Enabled correctors: confirmed hazards

All six below affect current `-a` and `-A` settings. The failure also exists in the
Ruby originals; the bare-hash case additionally has a Rust AST grouping difference.

| Priority | Rule | Confirmed break | Recommended guard |
| --- | --- | --- | --- |
| P1 | Both table-option rules | `create_table(:widgets)` becomes invalid `create_table(:widgets), charset: ...` | Insert into the actual argument list or refuse correction |
| P1 | NoSetex | Reordering TTL/value expressions changes their evaluation order and can swap stored data and expiration | Preserve evaluation order or refuse effectful/uncertain expressions |
| P1 | NoSetex | Swapping two heredoc openers leaves bodies/terminators in the original order, producing invalid Ruby | Treat complete heredoc units together or refuse |
| P1 | NoSetex | A splat becomes invalid `ex: *args` syntax | Refuse splats and uncertain arity |
| P2 | NoSetex | An implicit positional hash becomes keywords; the corrected `set` call lacks a value and raises | Preserve a complete positional hash with braces |
| P2 | Both table-option rules | Hardcoded defaults override charset/collation already supplied by `**options` | Refuse unknown keyword splats |

The evaluation-order failure is observable at runtime, not merely a style concern:

```ruby
events = [60, 1234]
redis.setex(key, events.shift, events.shift)
# Before: TTL 60, value 1234.
# After correction to set(..., value, ex: ttl): TTL 1234, value 60.
```

## Serializer correctors: blocked by current configuration

Both corrections are `AutoCorrect: disabled` in Wistia, verified under `-a` and `-A`.
If enabled elsewhere, they inherit four confirmed hazards from Ruby:

1. Moving `return` from a method into an attribute block raises `LocalJumpError`.
2. Singular correction drops optional method parameter bindings/defaults and can
   raise `NameError`.
3. Plural correction removes a conditional declaration's call but leaves its
   modifier `if`, producing invalid Ruby.
4. Plural correction uses a textual `def name` replacement; valid `def  name`
   spacing prevents conversion, yet the attribute declaration is removed,
   silently dropping the serialized key.

Keep these corrections disabled. Marking them unsafe would only protect ordinary
`-a`; arbitrary `-A` still needs semantic guards. Syntax checking alone would miss
three of these runtime/data failures.

## Diagnostic-only regressions

These cannot directly rewrite code, but six Rust differences weaken guardrails or
cause false lint failures under current config:

| Priority | Rule | Rust difference from Ruby |
| --- | --- | --- |
| P2 | NoSetWithoutExpiration | Misses pipelined/transactional `do/end` bodies; brace bodies work |
| P2 | NoCallbackSkippingProtectedWrites | An argument comment bypasses `update_column(:protected, ...)` checks |
| P2 | AnyCable/StreamFrom | An argument comment bypasses custom-coder checks |
| P2 | AnyCable/StreamFrom | A nested target-building `coder:` option is mistaken for a stream option |
| P2 | RescueMustReportError | Valid `Rails.error().report(error)` is rejected |
| P3 | RescueMustReportError | A local variable named `raise` is mistaken for re-raising |

## Recommended order

1. Fix suppression/edit ownership and corrected-result accounting in the engine.
2. Disable the three exposed custom correctors pending guards, preserving their
   diagnostics. This is a recommendation; no config has been changed by the review.
3. Add the adversarial CLI and runtime cases above before repairing or enabling
   corrections. Validate syntax after each candidate batch and refuse invalid
   rewrites; retain runtime tests for evaluation order and serializer behavior.
4. Fix the diagnostic guardrail regressions, especially missing expiration and
   protected-write checks.
5. Keep both serializer correctors disabled unless correction safety can be proved
   for a deliberately restricted set of method/declaration shapes.

Raw synthetic probe results are available in this review session under
`/tmp/rr-review-engine.json`, `/tmp/rr-review-correctors-*.json`,
`/tmp/rr-review-serializers-*.json`, and `/tmp/rr-review-guardrails-*.json`.
The linked reports retain minimal source reproductions and exact implementation
locations so the findings remain usable without those temporary artifacts.
