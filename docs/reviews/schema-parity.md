# Schema/database parity queue

This separate branch starts at framework checkpoint 71223ed and owns four Rails
no-op stubs: BulkChangeTable, NotNullColumn, ReversibleMigration and
UniqueValidationWithoutIndex. Completed RSpec and Performance branches remain
untouched. The JSON report tracks every owned rule, including unfinished work.

Pinned reference: Ruby 3.4.9, RuboCop 1.65.1, Rails cops 2.26.0, parser 3.3.5.0.
The nonexecuting Prism importer retains all 255 upstream source/config expansions
with zero unresolved templates. It re-evaluates inherited literal let definitions
and records explicit Rails-version tags, schema sources and database fixture data.
These fields for pending rules are evidence to replay, not implementations or
claims of parity. Imported code is never executed.

NotNullColumn matches 84 originals and 168 generated states exactly: diagnostics,
messages, ranges, severity/correctability and safe/all bytes/final state/idempotence.
All25 upstream sources are retained. Source review corrected the stub's warning
severity and narrow migration inclusion: original defaults are convention and all
db/**/*.rb. Explicit Severity and AutoCorrect controls are tested. The native
emitter applies configured severity; an engine-wide direct-node severity override
issue is reported separately to the parent integration task.

Original semantics distinguish AST type nodes from Ruby method-name Symbols:
add_column/column virtual types are exempt, but shortcut t.virtual is not; MySQL
text shortcut behavior likewise differs from an explicit text type node. Parenthesis
and shallow-body traversal controls preserve the original distinctions. Default:nil
still reports, while false/zero/other default values exempt. No correction is
supported, so all inspected source bytes remain unchanged.

The reusable database resolver follows explicit option, development YAML, then
DATABASE_URL precedence. Fifteen observations use the actual original file reader
inside owned temporary directories; tests cover supported adapters, multi-database
configuration, unsupported adapters, missing/invalid YAML, environment URLs and
YAML aliases. No ERB is evaluated. Reads use the existing per-lint-operation cache.
Actual production CLI scope passes 116 controls/51 rules with zero differences,
including both db/migrate and other db files and their model-path exclusions.

ReversibleMigration's first 48 upstream originals/96 generated states are exact.
It reuses the existing original-backed migration-class matcher through a thin
visibility wrapper. Expanded method/scope/version/body negatives and its own
production controls are still in progress; BulkChangeTable and schema-index
validation remain pending. All four originals support diagnostics without
corrections. Generic correction scheduling remains explicitly deferred.

## Final four-rule review

All four owned stubs are implemented. Final evidence contains 576 originals and
1,152 generated states. Whenever original diagnostics are available, counts,
messages, ranges, severity/correctability, safe/all bytes, final state and
idempotence match. The strict driver exits one solely for three states of the
original NoMethodError on bare drop_table (missing required DSL arguments). No
allowlist or altered Wistia rule configuration conceals that error.

| Rule | Originals | Generated states |
| --- | ---: | ---: |
| NotNullColumn | 84 | 168 |
| ReversibleMigration | 164 | 328 |
| BulkChangeTable | 249 | 498 |
| UniqueValidationWithoutIndex | 79 | 158 |

The distinct post-implementation source review checked ReversibleMigration's
ancestor/version gates, ordinary versus numbered block exemptions, original
shallow Ruby-AST body traversal and offense-registration order. The Rails-private
migration_body projection is shared with BulkChangeTable and uses the existing
canonical send projection. Bulk grouping follows literal value.to_s, consecutive
run boundaries, adapter/version lists and the original AST-value display for
tested command literals/interpolations. Commands are parsed, never executed.

UniqueValidationWithoutIndex uses the original schema AST model: table/column
lookup, existing and legacy add_index forms, expression substring matching,
unique-key presence semantics, model namespaces/table-name overrides, association
foreign keys/polymorphism, conditional validations, freeze scopes and typed word
arrays. Index-sensitive positive/negative contrasts caught and repaired word-array
projection; the existing shared word decoder is reused. Original schema fixture
hooks construct the actual original Schema from parsed source and reproduce the
upstream database-fixture context. Production reads use the existing per-operation
filesystem cache and never evaluate schema code or ERB.

Shared inflection work is isolated in 2adca41: 335 original pluralize/tableize
observations plus the preserved 285 underscore/foreign-key observations. Source
is pinned to ActiveSupport 8.1.3.1 default English rules with an empty acronym
registry. Broader runtime-custom inflections and exotic schema/command AST
families remain bounded evidence, as explicitly recorded in the JSON.

The narrow config fingerprint includes schema bytes when schema validation can
run, and only the normalized database adapter when database-dependent rules can
run. Credentials are never fingerprinted or written to artifacts. Real CLI tests
keep the native disk cache enabled and compare against fresh original Ruby results
through schema creation/deletion, unique-index changes, equal-size/equal-mtime
edits, YAML/environment adapter changes and absent/5.1/5.2/6.0/6.1 railties locks.
All 15 controls match. Ten owned production-scope controls across all four rules
also match, including the current Wistia schema. The Bulk adapter override is an
isolated fixture that inherits production config and preserves its path rules.

No original in this queue provides autocorrection, so there are no migration
rewrites to execute against a database. Source identity is checked in both modes.
No release, S3 publication or rollout-PR modification is part of this branch.
