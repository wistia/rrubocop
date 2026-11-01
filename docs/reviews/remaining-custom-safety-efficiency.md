# Remaining custom-rule safety and efficiency fixes

Follow-up to the review of `89b63121a3b3b4ddd24fb5354958ca1847d9c2f9`.

## Correction safety

The app-specific charset, collation, and NoSetex correctors had no newly confirmed defect. The additional hazards were in retained generic ports, and several reproduce in bundled Ruby RuboCop itself. The fork deliberately declines those corrections rather than reproducing destructive upstream rewrites.

MixinGrouping retains diagnostics but leaves effectful arguments, comments, and grouping across intervening statements manual. Accepted grouped rewrites use a canonical whole-span edit with duplicate-owner aliases, making the operation atomic when another cop conflicts with one member. Ordinary contiguous constant-based grouping/separation remains available.

UselessTimes leaves uncertain rewrites manual: trailing statements/comments, multiline literals and heredocs, implicit rescue/ensure, escaping block control flow, unsupported/implicit parameters, symbol block passes, and expression-value contexts such as arrays, hashes, arguments, and assignments. Deindentation only removes whitespace. Nested control targets and ordinary supported rewrites remain covered.

`Lint/UselessTimes` remains unsafe-only (`-A`). Removing a block can still change its return value or lexical scope; this pass does not claim general runtime equivalence for every unsafe correction. Ordinary constant lookup during MixinGrouping retains the conventional upstream assumption; effectful argument expressions are rejected.

## Diagnostic parity

Typed static-symbol/string helpers fix escaped and percent-literal values in queue keys, serializer checks, autoload requires, and Sidekiq expectations. Dynamic expressions remain unevaluated, ordinary literals borrow source text, and raw Context::value behavior remains unchanged for other consumers. Consumer-specific AST boundaries are preserved, including Ruby's treatment of chained require arguments.

Structural constant names fix multiline/root-qualified superclass, policy-scope, NSQ exclusion, and broad-rescue checks. Protected-write matching now enforces the Ruby argument shape, including multiple bare keyword pairs forming one hash and exclusion of extra positional arguments. Serializer method-moving ports remain diagnostic-only.

## Efficiency

- Diagnostics share source-line snapshots through Arc<str>, rather than copying a long line once per offense. Snapshots survive source replacement during autocorrection.
- Nonempty cache entries use an interned line table and indices; allocation identity avoids hashing the same long line for every diagnostic. Decoding restores sharing and rejects malformed tables. Legacy cache entries remain readable, and empty entries retain their compact envelope.
- Cache pruning reads timestamps without deserializing full diagnostic payloads.
- UselessTimes parameter patterns cache by value rather than source pointer; line-prefix checks use the existing tree byte column.
- Ancestor queries memoize strict parent context and build one iterative named-node index after 32 unique queries. Sparse files remain lazy. A first isolated deep query retains the earlier parent-lookup cost; the change targets repeated deep queries.

The public Rust Diagnostic.source_line field changes from String to Arc<str>. Its serde representation remains a string with empty omission, and CLI JSON, text rendering, and MCP shapes remain unchanged. Cache schema 2 is internal. Rules revision advances to 32 so older diagnostic/correctability results are not reused.

## Validation

Implementation: `cb936d2ea6760016186649678ebdcf8308761957`.

- 782 tests pass across all targets, including 2,205 pinned port references plus 30 earlier final-parity regressions (2,235 across the 98 ports).
- 83 new diagnostic cases replay original bundled Ruby cops without new diagnostic exceptions. Fifty-two retained safety cases record reasoned correction/correctability exceptions for harmful upstream behavior. Off/Safe/All coverage and annotated fixtures remain in place.
- The release CLI passes 156 safety comparisons (52 cases × three modes). All 52 expected outputs compile with RubyVM. Thirteen selected hazard runtime observations and seven stored-value runtime observations remain equivalent.
- Atomic grouped mixin corrections retain alias credit and pass combined-cop conflict and disabled-member tests.
- Cache coverage checks real warm hits, changed-content misses, shared excerpts, original snapshots after correction, legacy/corrupt formats, compact size, and public/MCP serde. A 256-offense × 10,000-character shared-line payload remains below 100 KB and more than 16× smaller than the old representation.
- Ancestor tests cover deep 600-level input, strict ancestor/self distinctions, cross-parse node queries, and sparse repeated lookups that never build the full index.
- 67 earlier CLI inspection probes and 21 earlier correction probes remain identical.
- Changed Rust formatting, diff checks, and ABC pass: zero function/module complexity offenses, 92 used-once advisories, zero never-used warnings. Clippy retains 223 preexisting diagnostics versus 224 previously; no new diagnostic groups. No suppression or threshold changes were added.

## Measurements

Same release profile and fixed corpus. Targeted scopes and the 632-file sample use three alternating repetitions per binary. Full runs use two each in before/after/after/before order. Processes run sequentially with warm filesystem and no competing builds/agent scans.

Read-disabled sample/full runs have isolated cache directories per binary/scope; they still write, preserving existing --cache false semantics. Other synthetic scopes use an unavailable backend except explicit cold/warm cache probes. Cache-cold disables reads and writes the entry; cache-warm enables reads on that same input and selection. Reported cache bytes are allocated database-file bytes, not logical JSON payload size, and vary with redb allocation history.

| Scope | Before median (s) | After median (s) | Before RSS (KiB) | After RSS (KiB) |
| --- | ---: | ---: | ---: | ---: |
| times-line-2000 | 0.132143 | 0.064945 | 93812 | 19944 |
| times-line-4000 | 0.374048 | 0.113463 | 323580 | 27112 |
| times-multiline-4000 | 0.118457 | 0.139314 | 27064 | 28316 |
| times-parameters-2000 | 0.145800 | 0.095392 | 39484 | 22316 |
| ancestors-nested-2 | 0.014158 | 0.014408 | 12852 | 13036 |
| ancestors-nested-600 | 0.182844 | 0.025270 | 15536 | 15784 |
| ancestors-flat-8000 | 0.063361 | 0.058955 | 21432 | 23004 |
| cache-cold-1000 | 0.145551 | 0.047286 | 119740 | 17636 |
| cache-warm-1000 | 0.068623 | 0.021324 | 65860 | 13160 |
| sample | 1.737251 | 1.725121 | 75084 | 75444 |
| full | 19.786399 | 19.898616 | 139046 | 137802 |

The long-line probes contain 2,000/4,000 separate `1.times` calls on one line; multiline has 4,000 calls on separate lines. Parameter probes repeat 2,000 `|i|` blocks. Ancestor probes have zero offenses: nested method chains at 2/600 levels and 8,000 flat instance-variable references inside a module. The explicit cache probe uses 1,000 calls on one long line.

The 4,000-offense same-line resident peak drops from about 316 MiB to 26.5 MiB. The 1,000-offense cold-cache peak drops from about 117 MiB to 17 MiB, and warm from 64 MiB to 13 MiB. Median allocated cache-file bytes fall from about 96 MiB cold / 64 MiB warm to about 2 MiB. These generated inputs demonstrate removal of duplicated line payloads, not typical application usage.

Tradeoffs are visible: the 4,000-line offense-heavy probe rises from 0.119s to 0.139s, with roughly 1.2 MiB more peak RSS, after added safety checks and shared-line bookkeeping. The flat ancestor case uses roughly 1.5 MiB more RSS for its index while becoming slightly faster. Sparse ancestor inputs remain close. The first isolated deep lookup still has the original parent-search cost.

All paired benchmark CLI offense arrays match. Full scans inspect 14,762 files and retain 27 findings: 19.786s before versus 19.899s after. CPU changes are similarly small; there is no demonstrated clean-corpus speedup. JSON output does not expose the correctable flag; deliberate safety changes to eligibility are covered by the strict reference tests, not inferred from JSON equality.

[Raw timings, allocated cache sizes, binary/fixture hashes, runtime observations, CLI checks, and runner](remaining-custom-safety-efficiency-measurements.json).

A [subsequent safety review](retained-autocorrect-guard-fixes.md) supersedes the constant-only MixinGrouping correction policy: the rule is now diagnostic-only, and further retained-rule guards cover scope, patterns, branches, and observable effects.
