# Deep audit of ten custom rules

This audit targets rules with comparatively sparse pinned examples or limited runtime coverage. Three agents tested and fixed the groups below against pinned RuboCop 1.65.1. The source baseline is 42f8ad3.

## Groups and findings

- **HashConversion and NoSetex:** HashConversion now separates argument nodes from attached blocks, preserves outer send blocks, inserts command-call parentheses before parent blocks, groups keyword splats, and distinguishes a local variable named `zip` from an argumentless method call. Zip calls with blocks or safe navigation follow the reference matching. Invalid upstream literal/block corrections stay report-only. No new NoSetex bug was confirmed; Redis-spy cases verify callback preservation and stable argument order, with existing effectful swaps left manual.
- **KeywordParametersOrder and RedundantWithIndex/Object:** Enumerator matching now recognizes actual numbered-parameter references in the correct lexical scope. Strings containing `_1`, `_10`, method selectors, bare alias/undef method names, and nested block parameters no longer trigger rewrites that drop side effects or change return values. Keyword corrections are explicitly tested with `AutoCorrect: true`, because project-disabled autocorrection otherwise leaves the rewrite unexercised. No new keyword-ordering defect was confirmed; dynamic defaults and comment safeguards remain.
- **ColonMethodDefinition, Dir, EmptyBlockParameter, MultilineWhenThen and ErbNewArguments:** Dir preserves outer blocks and excludes block-bearing inner calls. ERB diagnostics group keyword options like the reference AST, recognize keyword splats, and keep ambiguous or invalid positional/keyword/block-pass rewrites manual. Existing quoted option preservation is retained. No new correctness gap was confirmed in the other three syntax rules.

## Test scope

The scoped parity fixtures contain 281 cases (67 conversion/Redis, 117 keyword/enumerator, 97 syntax/ERB/path). Tests span enabled autocorrection, custom methods and coercion, duplicate keys, argument evaluation order, positional/keyword splats, callback forwarding, lexical scopes, nested blocks, source comments and correction spans. Intentional safeguards have precise per-example diagnostic/source expectations; no broad audit exemptions were added.

The original six-seed baseline passed 670 compilable comparisons before the targeted probes found the additional bugs. This illustrates why generated case count alone is insufficient to establish coverage. The deeper post-fix run also uses six mutation seeds per selected rule rather than the normal two.

A separate combined-rule suite exercises corrections together, including hash conversion inside keyword defaults and enumerators, singleton-method syntax, paths and multiline branches, and Redis-spy calls. Known behavior changes shared with upstream are recorded rather than treated as new native defects.

Cache rules revision is 42. New runtime programs are explicit, reviewed and self-contained; generated source is not authorized for execution.

The final alias/undef follow-up preserves interpolation: a method-name symbol such as `:"#{_1}"` still counts as a real numbered-parameter read. The helper ignores only bare alias/undef names. The full and deeper-ten-rule runs preceded this small follow-up; a final six-seed audit of both affected enumerator rules covers the old affected case identities and the eight added controls.

## Verification

- 823 Rust tests pass, including the new combined-rule regression; one batch-audit probe remains ignored in the normal suite.
- The central corpus contains 3,278 references, plus 30 separate pinned references. Scoped fixtures include one invalid top-level yield example; compilation-invalid examples remain exclusions rather than passing comparisons.
- Full audit: 5,016 selected examples, 4,976 compilable comparisons, 98 cops, zero new failures; 48 explicit runtime scenarios across 25 cops.
- Deeper ten-rule audit: 1,001 selected examples, 988 compilable comparisons with six seeds per rule, zero new failures; 25 runtime scenarios across all ten targeted cops.
- Final alias/undef follow-up: 221 compilable examples across both enumerator rules, zero new failures. Every affected enumerator identity from the full run is included.
- Seven combined-rule programs run in both engines and all three modes (42 engine/mode checks); 21 native mode outputs are pinned as normal-suite regression cases. Existing keyword-default safeguards differ in source as intended; the observed with_object return-value change matches upstream and is not newly suppressed.
- Real release CLI: 198 valid conversion/Redis checks, 351 keyword/enumerator checks, and 291 syntax/path/ERB checks. Additional actual Ruby CLI and runtime comparisons verify selected controls and fixed points.
- Formatting and ABC checks pass; the existing Clippy backlog is unchanged.
