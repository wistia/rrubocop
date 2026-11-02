# Runtime coverage: migration options, requires, strings and constants

This batch adds 26 explicit trusted scenarios for six rules: CreateTableMustIncludeCharset, CreateTableMustIncludeCollation, RedundantFileExtensionInRequire, TrailingCommaInBlockArgs, TripleQuotes and DeprecatedConstants.

Migration programs use self-contained method spies, not a database. They verify existing keyword values and metadata, block invocation and return values, expected default options when supplied, dynamic option-bundle safeguards and parenthesized edit boundaries. Require programs provide a local sandbox file and check load-once behavior and encoded filenames. Block programs check destructuring, lambda calls and chained blocks. String programs compare bytes, encoding and interpolation effects. Constant programs check builtin-style aliases and configured namespace replacements, with an unmapped control.

The runtime programs identified no new native-only correctness issue. The broader six-seed pass reached six previously unpinned forms of existing intentional differences: four migration corrections where Ruby appends outside parentheses and generates invalid syntax, and two programs combining encoded and ordinary require extensions. Each form now has exact source and diagnostic expectations; ordinary literal filenames still correct while encoded filenames remain manual. No broad audit exemption or production-rule change was added.

All original runtime programs execute successfully. Ruby's invalid migration corrections and encoded-filename corruption remain separately classified upstream hazards; valid native corrections and existing safeguards preserve the tested behavior. Generated variants are not authorized for runtime execution.

## Verification

The six-rule audit with six mutation seeds per rule covers 555 selected examples, 537 compilable comparisons and 26 runtime programs, with zero new failures. Invalid originals and mutations remain separate classifications. The normal suite passes 823 Rust tests (one ignored batch probe), including 3,365 central references plus 30 separate pinned references. All 96 release CLI checks for the 32 scoped rows pass.

Unified runtime coverage now contains 127 explicit scenarios across 43 of the 98 added rules. This focused run executes the 26 scenarios for the selected rules; earlier unchanged scenarios were verified in previous batches. No production code changed in this batch.
