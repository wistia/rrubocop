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
