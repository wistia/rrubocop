# Independent parity audit follow-up

The follow-up findings recorded here were addressed in [the discrepancy resolution](discrepancy-resolution.md). The unresolved artifact below is the historical input to that work.

This follow-up addresses the gaps found by the independent reviewer after the previous batch run was clean. It expands the regression corpus and generators so the clean result is tested against more syntax and encoding shapes.

## Corrections and argument shapes

UnpackFirst excludes unpack calls with attached or forwarded blocks and unsupported argument shapes, matching pinned Ruby. This preserves the block's side effects and the original return/exception behavior. Corrections of an outer accessor preserve its attached block.

The remaining semantic-argument consumers skip comment nodes when selecting operator, accessor, conversion, require, mixin, and block arguments. Existing HashExcept and MapCompact correction safeguards remain in force.

## Literal identity

Duplicate checks compare typed literal values recursively rather than only source spelling. Numeric spellings, nested collections, symbols, and binary string escapes are covered. Duplicate keys retain byte/encoding distinctions for non-ASCII values, including binary source strings versus Unicode-escaped strings. The shared general-purpose string decoder retains its previous behavior; byte decoding for duplicate keys is scoped separately.

The explicit encoding runtime scenario confirms that the Unicode case branch remains reachable. The unpack runtime scenario confirms that the original block executes and the original exception path remains observable in safe and unsafe correction modes.

## Audit coverage

The generator additions target argument positions throughout AST calls, operator-call syntax, empty accessor arguments, attached blocks, numeric representations, nested literal equivalents, and source encodings. Mutations remain deterministic and bounded by the seed budget. Generated programs receive no runtime execution authorization; behavior checks remain explicit trusted scenarios.

Cache rules revision is 39.

## Verification and unresolved follow-up

The final Rust suite passed 820 tests with one ignored test. All 2,858 central pinned references pass after adding 51 semantic-argument and 60 literal-key regressions. ABC checks report zero function or module complexity offenses. Clippy remains blocked by the existing lint backlog (223 test-target diagnostics).

The expanded live audit selected 4,573 examples across all 98 cops, with 4,534 compilable examples and 13 trusted runtime scenarios across 11 cops. It found **153 new discrepancy records**, including diagnostics, correction-mode differences, and native errors; these are unresolved and this audit is not clean. Counts are records, not distinct root causes. The expanded run preceded the final 60-case central-fixture append.

| Cop | Discrepancy records |
| --- | ---: |
| `Style/RedundantConditional` | 24 |
| `Style/SlicingWithRange` | 22 |
| `Style/HashExcept` | 18 |
| `Cops/CreateTableMustIncludeCharset` | 13 |
| `Cops/CreateTableMustIncludeCollation` | 13 |
| `Lint/SafeNavigationConsistency` | 12 |
| `Style/UnpackFirst` | 9 |
| `Style/ClassEqualityComparison` | 6 |
| `Style/OrAssignment` | 6 |
| `Style/RandomWithOffset` | 6 |
| `Lint/TrailingCommaInAttributeDeclaration` | 5 |
| `Lint/IdentityComparison` | 3 |
| `Lint/MultipleComparison` | 3 |
| `Lint/SafeNavigationWithEmpty` | 3 |
| `Lint/DuplicateElsifCondition` | 2 |
| `Lint/RedundantWithIndex` | 2 |
| `Lint/RedundantWithObject` | 2 |
| `Style/TrailingCommaInBlockArgs` | 2 |
| `Lint/FloatComparison` | 1 |
| `Security/CompoundHash` | 1 |

The unresolved generated inputs and exact differences are retained in [independent-parity-unresolved.json](independent-parity-unresolved.json); 153 records correspond to 64 selected cases. All 38 Python harness tests pass with the bundled Ruby project, including the generator integration checks.
