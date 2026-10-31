# Independent regexp port review at 832a13f

Read-only review; no repository files changed or cargo/full corpus runs. Compared pinned Wistia RuboCop 1.65.1 Commissioner on Ruby 3.4.9 with ready release rrubocop binary. Added 67 temporary probes: 59 valid Ruby cases, 8 invalid patterns excluded. Machine evidence: `/tmp/rr-five-final-regexp-probes.json`; generation and execution scripts beside it.

## P2 — Restore Ruby evaluation order for deferred heredoc bodies

Location: `src/cop/ported/difficult/range/walk.rs:35` (also generic walk at line8 and binary handling at line52).

Tree-sitter stores heredoc bodies outside the call's children. `call` finishes its after-send capture reset before visiting their interpolations, whereas Ruby processes interpolated argument expressions first. Both missed diagnostics and false positives result.

False positive:
```ruby
"x".match(<<~TEXT)
 #{/(a)/ =~ "a"}
TEXT
puts $2
```
Pinned Ruby emits no offense (outer dynamic match resets capture knowledge after its argument); native warns at line4,column5 `$2 is out of range (1 regexp capture group detected).`

False negative:
```ruby
"x".gsub(/(x)/, <<~TEXT)
 #{$1}
TEXT
```
Ruby warns at line2,column3 `$1 is out of range (no regexp capture groups detected).`; native emits nothing. Also reproduced with a previous literal match followed by dynamic `match(heredoc)` and dynamic `r =~ heredoc`.

Recommendation: associate deferred bodies with their heredoc argument nodes and visit each interpolation before applying its enclosing call/binary after-state. Avoid processing the body again later. Add pipeline tests for both directions, including multiple heredocs and nested interpolation matches.

Coverage gap: both snapshot cops currently have zero heredoc cases; prior extra regexp probes also omit heredocs.

## P2 — Do not treat an element assignment target as a `[]` send

Location: `src/cop/ported/difficult/range/walk.rs:57` (unconditional `after("[]", ...)`).

```ruby
/(a)/ =~ "a"
s[/(a)(b)/] = $2
puts $2
```
Ruby emits warnings on both `$2` occurrences at line2,column14 and line3,column5, using the existing one-capture state. Native emits none: it incorrectly changes the count to2 while visiting the LHS element_reference. Ruby's actual call is `[]=`, which is outside the cop's restricted methods. With `$3`, native does report but uses the wrong two-capture message instead of one.

Recommendation: identify element references in assignment target position and suppress the synthetic `[]` after-state; cover plain and compound element assignment independently against Ruby.

## Remaining review results and safety

No additional discrepancy in 53 matching valid probes, including named/numbered groups, lookarounds, atomic/absent groups, conditional captures, inline extended flags, comments, character classes/intersections, escaped parentheses/control escapes, named backreferences/subexpression calls, case guards, local names and nested calls. No panic observed. Neither reported issue is an autocorrect issue: both ports are intentionally diagnostic-only.

Existing `/tmp/rr-five-cli.json` already proves actual CLI `-a` and `-A` leave source unchanged for both cops, including explicit AutoCorrect configuration; registration declares no supported correction. The 375-case pipeline snapshot compares line/column/message/correctability; it does not independently prove highlight lengths. Existing scanner/model tests and prior regexp-extra probes were inspected; no broad suite was rerun. This targeted review does not claim exhaustive legacy escape/class regression coverage.

Known shorthand interpolation safeguard (`#$value` makes static count unknown) was excluded from findings, as requested. Source-order conditional behavior matching upstream Safe:false was also not flagged.
