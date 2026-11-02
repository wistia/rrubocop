# Runtime correction coverage metrics

The historical runtime coverage fields counted scenario presence, including negative controls and report-only guards. An independent review checked actual final release output and reconciled six saved audit runs: 155 scenarios covered 49 rule names, but only 40 rules had completed positive rewrites. Eight source-rewriting rules lacked a positive execution; the ninth scenario-only rule was diagnostic-only DuplicateCaseCondition. The distinction includes SafeNavigationWithEmpty, whose sole rewrite raised an exception shared with Ruby and therefore did not complete normally.

The artifact-derived baseline is retained in runtime-activity-baseline-summary.json. It is historical, combining the listed saved runs rather than rerunning the current binary. The recorded mode counts are 77 safe and 101 all-mode positive rewrites, with three blocked scenarios in each mode. New positive examples and production fixes are integrated and re-audited separately.

## New reporting

`runtime_activity` is loaded from native.jsonl, reference.jsonl and runtime.json. A positive rewrite requires actual changed source, valid original/native compilation, normal original/native execution, no native/runner error and no native-only behavior regression. Static fixture correction text is not used as evidence that a correction ran. Reference-only failures can coexist with a successful native rewrite preserving the original behavior.

Per-mode and per-cop counts distinguish positive rewrites from unchanged negative controls, unchanged report-only diagnostics, unchanged corrections not applied in that mode and blocked/unverified runs. Behavior outcomes separate preservation, preservation despite a changed reference, shared reference changes, native-only failure, runner failure and missing observations. Shared exceptions are recorded as behavior changes but remain blocked from positive coverage.

Existing presence and uncovered-runtime fields remain compatible. The additional status identifies complete, partial, unavailable, skipped and not-requested artifacts; no valid execution is inferred from missing files or modes. Duplicate, unexpected or incomplete runtime observation records fail instead of inflating counts. Positive activity is separate from the audit's diagnostic/source parity failures and is not a claim of general cop correctness.

Regression tests cover the seven originally unchanged-only rules, mode-specific correction, failed and invalid compilation/execution, shared exceptions, native-only behavior changes, reference-only failures, missing artifacts, skipped runtime and report integration. Raw non-UTF-8 fixture tests remain separate from the Unicode JSON audit corpus.
