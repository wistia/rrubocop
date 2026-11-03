# Final Rails parity review

This branch implements Rails/Delegate, Rails/FilePath and
Rails/HttpPositionalArguments against RuboCop Rails 2.26.0, RuboCop 1.65.1,
parser 3.3.5.0 and Ruby 3.4.9. The original algorithms are in
`rubocop-rails-2.26.0/lib/rubocop/cop/rails/{delegate,file_path,http_positional_arguments}.rb`;
Delegate also uses RuboCop's `cop/mixin/visibility_help.rb`.
The 125 imported examples come from the upstream v2.26.0 specs and retain their
MIT license. The static Prism importer resolves every example without evaluating
upstream Ruby. Reimporting after Ruby lint produces identical fixture data.

The distinct post-implementation review checks Delegate's exact forwarding
argument shape, receiver and name/prefix matching, ordinary versus singleton
methods, safe-send exclusion and inline/prior/later sibling visibility. The
original default config has no controller exclusion; the inherited stub's extra
exclusion was removed. Method bodies with comments retain the diagnostic but
are not rewritten, preserving source information that the original deletes.
Delegation changes arity/introspection inherently; no universal behavior claim
is made beyond the trusted ActiveSupport observations.

HTTP review checks all six verbs, logical implicit-hash grouping, keyword/format
exceptions, sole keyword splats, receiver exclusion, ordinary versus numbered
routing blocks and file-wide Rack::Test inclusion. NodePattern `!nil?` excludes
missing Ruby objects, not a NilNode path: explicit `get nil, ...` still reports.
Both actual railties gem requirements and explicit TargetRailsVersion gate this
pinned original cop. Eight real-file CLI controls exposed the Commissioner
harness omission: original Team still checks the target minimum because its
requires-gem marker namespace differs from Rails 2.26.0. The harness now delegates
to the actual original Team predicate; HTTP applies both gates. Absent/lower and
conflicting target/lock versions are tested.
Corrections preserve action/params/session source and attached blocks. Native
refuses original rewrites that drop extra middle arguments, mixed keyword splats
or comments. Those correctability/byte differences remain strict failures.

FilePath review checks both styles, absolute-qualified Rails/File constants,
recursive Rails.root search, root arity, call argument types, interpolation slash,
colon and extension branches, first-child search and original first-line range
semantics. Shared canonical send ranges exclude attached blocks. Existing Ruby
literal decoding/inspect formatting is reused. Native declines literal activation,
source-comment loss and absolute-path transformations that alter Pathname results.
Additional Pathname probes show that interior `..` and duplicate separators
remain unchanged; those cases retain the original correction behavior.
File.join array arguments and omitted parentheses expose original runtime
failures or result-class changes, so native declines those edits. Percent-string
delimiters expose an original correction loop; the stable native
split remains an explicit difference. Original misplaced Rails.root File.join and
bare-root extension cases raise original exceptions; native remains stable.

The strict driver checks full diagnostic metadata and exact safe/all bytes, then
checks both generated states, compilation and idempotence. It has no allowlist.
All residual failures and their original/native observations are retained in
`final-rails-parity.json`; correction guards also change correctability metadata.
Only explicitly authored runtime programs are executed. They observe real
Pathname and ActiveSupport delegation, and a version-adapting HTTP call recorder
with evaluation order and state. Imported examples are only parsed/compiled.

Eleven actual production CLI controls cover all three rules, model/controller/
service/spec/test paths, outside-spec suffixes and disable directives. They match
with no missing positive controls. Native library tests pass 934, with one ignored.
The tiny HttpStatus diff is inherited rustfmt-only formatting. The importer lints
clean under the actual Wistia config and its data round trip is identical.

Remaining boundaries are explicit: arbitrary dynamic/heredoc/byte-encoding AST
families are bounded evidence, not universal equivalence. Generic combined-cop
scheduling remains deferred by user instruction. No release, S3 publication or
rollout-PR changes belong to this branch.

The refreshed reference gate uses the actual original Team predicate (isolated
commit `22ee3d2`) as well as Base gem requirements. All eight physical lock/target
controls match. Ruby lint retains one inherited RescueMustReportError finding
in the JSON exception-reporting harness; the new importer has no offenses.

Final gate: 547 originals and 1,094 generated safe/all states. Delegate has 145
originals, FilePath 134 and HttpPositionalArguments 268. Detection, message,
range and severity differences are zero. There are no invalid native corrections
or idempotence differences. All 26 authored programs preserve behavior in 52
native modes. Ten original/native runtime differences demonstrate the protected
absolute-path, array and omitted-parentheses hazards.

The strict driver retains 316 failures: 96 correctability-only diagnostic
observations, 99 safe-byte and 99 all-byte observations, 12 original exception
states and ten runtime observations. The only byte differences without a
correctability guard are three original percent-delimiter correction loops.
These counts include generated states and are not distinct-rule counts.
