# Production matcher reconciliation

The 091dc33 real-repository comparison exposed four RSpec/Be and thirteen
GraphQL/PrepareMethod Rust-only offenses. The fixes follow the pinned original
algorithms; configuration availability and user policy are preserved.

RSpec 3.0.4 Be matches a plain argumentless `be` send. In `to be {}` the matcher
is instead a block wrapper, so it must be excluded; a do/end block attached to
the outer runner still leaves a plain `be` and remains flagged. The added 27
original cases cover these bindings, parameters/numbered blocks, local names,
receivers, safe sends, comments, argument/block passes and parentheses.

GraphQL 1.5.4 PrepareMethod matches only the final pair of the final hash, with a
static symbol key named prepare. Wistia's flagged calls all have later options.
The native matcher now uses existing logical argument grouping and typed symbol
projection, preserves bare-call receiver restrictions, handles true block/const
values and reports their exact first-line range. Its 43 original cases include
positive controls, comments, escaped keys, explicit/implicit hashes, splats,
duplicate keys, arbitrary positional arguments, block and constant values,
Unicode/CRLF and each message-style branch. The optional string/symbol-style
correction capability remains an explicit separate review gap.

Both rules were rechecked against original source after implementation. Fresh
pinned Ruby references and the native pipeline agree on all 70 cases and 210
modes for counts, messages, ranges, severity, correctability and bytes. All
sources stay identical in off/-a/-A modes, so final offenses and idempotence are
unchanged. Source SHA256 provenance and remaining production gates are recorded
in production-matcher-reconciliation.json. No unsafe rewrite or detection
exception was added. Actual full-file reconciliation follows the fresh build.
