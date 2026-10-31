# Independent EvalWithLocation review (832a13f)

Read-only review of src/cop/ported/difficult/eval.rs and eval/location modules. Compared actual release CLI against bundled RuboCop 1.65.1 on Ruby 3.4.9, isolated cop/config. New probes run with both -a and -A; raw records: /tmp/rr-five-final-eval-probes.json and /tmp/rr-five-final-eval-probes-unsafe.json. No repository edits or cargo invocations.

## P2: Command heredocs are falsely considered literal Ruby source and safe-corrected

Location: src/cop/ported/difficult/eval.rs:32-33 (literal_code).

Before:
```ruby
class_eval <<`CODE`
printf 1
CODE
```
Native -a and -A produce:
```ruby
class_eval <<`CODE`, __FILE__, __LINE__ + 1
printf 1
CODE
```
Pinned Ruby reports zero offenses and leaves it unchanged. Backtick heredocs are xstr command execution: their evaluated string comes from shell output, not the literal heredoc text. A command can emit arbitrarily many differently located Ruby lines, so assigning this body's physical line as the generated Ruby location is unjustified. Native regular backtick expressions are excluded, making the heredoc treatment inconsistent too. This is a confirmed false positive with an applied safe correction, not a documented safety refusal. Scope: command heredoc passed to any eligible eval method. Existing reference cases cover string heredocs, not command heredocs. Fix: exclude backtick-quoted heredoc_beginning nodes (Ruby xstr/dxstr equivalents) from literal_code; add diagnostics and -a/-A no-change regressions.

## P3: Valid string literal forms are silently missed

Location: src/cop/ported/difficult/eval.rs:32-33.

Before `eval(?1, binding)` stays unchanged with zero native offenses. Pinned Ruby reports/corrects to `eval(?1, binding, __FILE__, __LINE__)`. The character literal is a valid one-character Ruby string containing executable Ruby. Likewise `eval(__FILE__, binding)` is skipped natively but corrected to `eval(__FILE__, binding, __FILE__, __LINE__)` by Ruby. These are literal-shape parity omissions, not justified conservative refusals (there is no finding/reason). Existing cases include quoted/percent/chained strings but neither character nor magic-file code argument. Scope: eligible methods receiving these source forms. Fix: recognize tree-sitter character nodes and the __FILE__ magic literal without broadening arbitrary identifiers. Add fixture and correction tests. Low frequency, no unsafe edit.

## Conservative correction limitation (not an unsafe-correction finding)

Location: src/cop/ported/difficult/eval/location.rs:47-55.

Native diagnoses `class_eval("1", (__FILE__), __LINE__)` and `class_eval("1", __FILE__, (__LINE__))` but refuses to edit their parenthesized argument. Ruby normalizes to bare magic literals under both flags. The refusal fits the implementation's strict expression-kind allowlist, though these particular pure wrappers can safely be unwrapped. Consider supporting a single expression inside parenthesized_statements and recursively checking purity. Do not broadly allow arbitrary begin/sequence expressions: that would discard side effects. This causes -a to leave an outstanding offense (exit 1 versus Ruby exit 0), but is lower priority than the confirmed false correction.

## Additional assurances, without duplicating prior coverage

New targeted probes matched native and Ruby for nested same-label heredocs, nested repeated labels plus multiple pending heredocs, a quoted escaped delimiter, percent string literal, parenthesized source literal (both deliberately ignored), and __ENCODING__ (both ignored). All inputs were independently ruby -c checked before comparison. Both correction modes gave the same results for the discrepancy cases.

Previously established and not redundantly rerun: 68 eval reference cases; pipeline diagnostic coverage; runtime/backtrace and refusal probes in /tmp/rr-five-cli.json; 657 passing Rust tests; atomic correction edit groups and syntax validation; disables/config support; syntax checks on 585 changed Ruby outputs. The four other new ports are diagnostic-only. Intentional refusals for unknown/effectful location expressions, splats/hash/block arguments, unsupported arity and missing eval binding are not findings. No issue found with the tested duplicate/escaped-label source ranges; no claim of exhaustive runtime parity.
