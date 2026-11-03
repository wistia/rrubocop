# Schema parity handoff

Branch: `feat/wistia-parity-schema`, based on framework checkpoint `71223ed`.
Earlier completed RSpec and Performance branches remain unchanged.

All four transferred rules are implemented and separately source-reviewed against
RuboCop Rails 2.26.0 with RuboCop 1.65.1/parser 3.3.5.0 on Ruby 3.4.9.
See [the review](schema-parity.md) and [machine-readable evidence](schema-parity.json).

Final gate: 576 originals and 1,152 generated correction states. No diagnostic,
metadata, corrected-byte, invalid-correction or idempotence differences on valid
original inputs. Three retained error states reproduce the original
ReversibleMigration crash for bare `drop_table`; native remains stable.
All four original cops have no autocorrection, so both correction modes preserve
source. No migration or schema fixture code is executed.

Native suite: 937 passed, one ignored. Owned production scopes: 10 controls across
four rules, zero differences. Real disk-cache gate: 15 controls, zero differences,
including same-size/same-mtime schema changes and railties lock versions.
Database resolution has 15 original properties; pluralization/tableize has 335
and preserves 285 underscore/foreign-key observations.

Shared prerequisites are included in this branch: canonical send projections,
original migration-class matcher wrapper, Rails-private shallow body projection,
default English inflection extension (isolated `2adca41`), schema AST model,
original literal-value rendering and a thin existing String#inspect formatter API.
The config fingerprint includes schema bytes and normalized adapter enum only;
credentials and URLs are never fingerprinted.

Integration should union schema_cases/schema_scope imports and the isolated
reference/schema_fixtures hook with newer framework harness changes. Preserve
other agents' fixture unions and the newer JSONL split fix. The tiny inherited
HttpStatus diff is rustfmt-only. Four new Ruby scripts lint clean; the inherited
JSON reference harness retains its existing RescueMustReportError lint finding.

Boundaries remain explicit: custom runtime inflection registries, non-UTF8
literals, executable/arbitrary schema DSLs and exotic command-interpolation AST
rendering are not universal-equivalence claims. Generic combined-cop scheduling
remains deferred by user instruction. No publication, S3 or rollout change.
