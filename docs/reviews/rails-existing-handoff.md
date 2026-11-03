# Existing Rails review handoff

Branch `feat/wistia-parity-rails-review`, based on framework `b21d03f`, repairs
ActiveRecordAliases, ActiveSupportAliases, ApplicationController, ApplicationJob,
ApplicationMailer, ApplicationRecord, BelongsTo, HelperInstanceVariable,
RakeEnvironment and ReadWriteAttribute. This is ten existing-rule reviews and
zero additional baseline-stub implementations. Earlier completed branches stay
unchanged. The implementation checkpoint is `6a1d5c1`.

The final combined gate has 1,649 originals and 3,298 generated correction states,
including all 131 statically imported upstream examples, the 40-case independent
safety matrix and 26 authored runtime programs. There are zero returned diagnostic
location/message/range/severity differences, invalid native corrections or
idempotence differences. All 52 native runtime modes preserve behavior.

The strict driver deliberately retains 298 differences: 108 correctability-only
metadata observations, 108 all-mode and 69 safe-mode byte observations, six
original Rake error states and seven runtime observations demonstrating unsafe
original corrections. See `rails-existing-parity.json` for every observation;
there is no allowlist. Original errors cannot supply comparable diagnostics.

Fresh native suite: 935 passed/one ignored. Actual production scopes: 31 controls
across all ten rules, exact. Physical target/lock versions: 32 controls across the
four minimum-version rules, exact. New Ruby importer: clean lint under the actual
Wistia config, and its complete 131-case data round trip is identical.

Integration: union the isolated `rails_review_*` case/runtime/scope/lock modules
and their two small framework harness import hooks. Keep other agents' fixture
unions and newer harness changes. The only shared code addition is a thin
`ported::constant_name` wrapper around the existing Context implementation;
Application superclass enforcement shares it with the FormBuilder matcher.
No shared decoder/model/engine was replaced. The inherited HttpStatus diff is
rustfmt-only. Rake's original Safe:false default is explicit in its fixture
profile and remains independent from SafeAutoCorrect, matching real config.

The source-review table and limits are in `rails-existing-parity.md`. Unsafe
opt-in superclass/alias/environment policy is retained; neutral trusted programs
do not establish equivalence for arbitrary custom APIs or inheritance logic.
Contextual autocorrection, non-UTF8/exotic dynamic syntax and combined-cop
scheduling are not universal claims; the scheduling engine is user-deferred.
No release, S3 publication or rollout modifications are included.

Important generic follow-up: the separate physical safety CLI runner has three
valid combinations exact and one retained original-invalid-input rejection
mismatch. Ruby rejects Safe:false + SafeAutoCorrect:true with exit 2; native
accepts it with exit 1/no edits. See the JSON physical_safety_cli field and
scripts/framework_parity/rails_review_safety_cli.py (strict exit 1 until fixed).
Original validator source: config_validator.rb:241 reject_conflicting_safe_settings.
The direct-cop 40-case matrix intentionally bypasses CLI validation and does not
claim that the invalid combination is a valid CLI config. Parent was notified;
no generic config-loader changes are included in this disjoint cop review.
