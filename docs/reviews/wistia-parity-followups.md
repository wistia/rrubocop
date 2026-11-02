# Deferred Wistia parity follow-ups

## Combined-cop autocorrection scheduling

Status: deferred at the user's explicit request. Keep this outside the current
per-rule implementation batches; implementing individual cops does not resolve it.

Pinned reference: `rubocop-rspec` 3.0.4,
`lib/rubocop/cop/rspec/let_before_examples.rb:54` declares
`autocorrect_incompatible_with` as `[RSpec::ScatteredLet]`. Ruby RuboCop uses
this metadata when choosing which cops may autocorrect together. The current
Rust cop/engine interface has no equivalent incompatibility metadata or scheduling
support identified by the audit.

This is a combined-rule execution gap, not evidence that either individual rule's
detection or isolated correction is wrong. Isolated Ruby differential tests and
idempotence checks must not be presented as proof of combined-cop parity.

When revisiting this work:

- Inventory incompatibility declarations across all pinned, enabled Ruby cops.
- Trace the pinned Ruby engine's scheduling and reinvestigation behavior.
- Reproduce the `LetBeforeExamples`/`ScatteredLet` interaction with both enabled
  under production configuration, including comments and intervening declarations.
- Compare `-a` and `-A` bytes, remaining offenses, correction passes, termination,
  idempotence, and relevant execution-order behavior.
- Add equivalent Rust metadata/scheduling and permanent combined-cop regressions.

Preserve any reproduction evidence in the RSpec agent's parity report. Do not
suppress these rules or add parity exceptions to hide the deferred engine gap.
