# Existing Rails parity review

This branch reviews and repairs ten existing implementations. It adds no new
baseline-stub implementation count. Base: framework `b21d03f`; earlier completed
RSpec, Performance, Schema and final-three-Rails branches are unchanged.

The reference is Ruby 3.4.9, RuboCop 1.65.1, rubocop-rails 2.26.0,
rubocop-ast 1.32.3 and parser 3.3.5.0. Source algorithms are the corresponding
`rubocop-rails-2.26.0/lib/rubocop/cop/rails/*.rb` files. Application enforcement
uses Rails' `lib/rubocop/cop/mixin/enforce_superclass.rb`, which replaces the
similarly named deprecated core mixin. ReadWriteAttribute's literal shadowing
also uses rubocop-ast's `DstrNode#value`; its concatenation/source behavior is
reused through the existing `original_string_value` helper. Constant matching
reuses the existing Context constant decoder via a thin wrapper.

All 131 upstream v2.26.0 examples are statically imported with no unresolved
examples; upstream Ruby is never evaluated. Original spec files and their MIT
license are retained. Only authored observation programs are executed.

| Existing rule | Source-confirmed repairs and reviewed branches |
| --- | --- |
| ActiveRecordAliases | Argument requirement, selector range, safe-send callback, exact warning/default and configured severity, disabled correction and independent safety flags. |
| ActiveSupportAliases | Typed Str/Array receivers, exact one-logical-argument matcher, ordinary/safe sends, selector-through-send range, append diagnostic without correction, prepend/unshift correction, dynamic/chained/physical string negatives. |
| ApplicationController | Exact constant namespace, own-base name exemption, named/anonymous Class.new, root versus namespaced assignment exemptions, exact range/metadata, unsafe correction and source-comment safeguard. |
| ApplicationJob | Shared superclass algorithm plus original message and both independent minimum-version gates. |
| ApplicationMailer | Shared algorithm, message, version gates, removal of an unsupported mailer-only default include. |
| ApplicationRecord | Shared algorithm, version gates, removal of an unsupported database default exclusion. |
| BelongsTo | Final logical hash only, typed symbol and literal boolean matcher, ordinary-send scope, selector highlight, exact true message, both version gates, later-optional precedence and comment safeguards. |
| HelperInstanceVariable | Reads and writes exactly once, assignment-left-only ||= exclusion, nested classes checked unless any class ancestor inherits the exact FormBuilder constant, operators/multiple assignment/interpolation and path scope. |
| RakeEnvironment | Ordinary block/numblock distinction, default-name and dependency shape, logical argument arrays/hashes, original unsafe correction, precise send range, source-preserving invalid-label/name-mangling safeguards, exact file paths. |
| ReadWriteAttribute | Implicit receiver only, exact arity/messages/ranges, ordinary-def-only decoded literal shadowing (including chained strings), source-preserving comments, assignment precedence/receiver/keyword-hash safeguards. |

The distinct post-implementation source review checks matcher cardinality,
receiver and constant roles, Ruby AST versus CST containers, safety/config
branches and correction ownership. Shared Argument projection, cached numbered
binding metadata, original string decoding and constant names are reused.
No generic autocorrect scheduling engine is changed.

At the first complete milestone, 1,609 originals and 3,218 generated states had
zero detection/message/range/severity differences. All 26 authored programs
preserved exit status, stdout, evaluation order and state in 52 native modes.
The original corrected bytes/runtime differ deliberately where native guards
preserve source or semantics. The strict driver has no allowlist: every residual
metadata/byte/error/runtime difference is retained in machine-readable evidence.
Examples include later optional options overriding converted required options,
write-attribute receiver/boolean precedence and Rake's mangled task names or
invalid hash/label rewrites. Native remains stable for original Rake exceptions.

The final independent safety-option matrix has 40 originals/80 generated states
and no differences. Rake's original `Safe: false` default is modeled explicitly
in its fixture profile, separately from SafeAutoCorrect. Both flags' independent
overrides are checked for every rule. This preserves real configuration semantics
instead of conflating the two flags in a native trait default.

Thirty-one actual production CLI scope/directive controls cover all ten rules,
with no differences or missing positive controls. Thirty-two physical lock/target
controls cover ApplicationJob/Mailer/Record and BelongsTo: absent/lower/supported
railties and contradictory explicit target values. Both original Team's target
predicate and actual gem requirements apply. Those controls all match.

Unsafe opt-in superclass/alias/environment transformations retain their original
configuration policy; neutral trusted observations do not prove universal runtime
preservation for custom application superclass logic, custom API overrides,
missing ActiveRecord attributes or loading a real application environment.
Arbitrary executable task code is not evaluated. Non-UTF8 and exotic dynamic
literal/macro AST families remain bounded evidence, not a universal-equivalence
claim. Contextual autocorrection beyond the tested boolean/disabled policies and
combined-cop scheduling are not claimed; scheduling remains user-deferred.
