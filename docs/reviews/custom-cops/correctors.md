# Custom corrector review

Reviewed commit `586bb4c`, read-only. Rules: `Cops/CreateTableMustIncludeCharset`, `Cops/CreateTableMustIncludeCollation`, and `Cops/NoSetex`.

All 3 are enabled today, without `AutoCorrect: disabled` or `SafeAutoCorrect: false`. The two table cops apply to `db/migrate/*.rb`; NoSetex is broadly enabled. Rust registers all 3 with safe autocorrection, so these findings are reachable through ordinary `-a`, as well as `-A`. Wistia currently has no new offenses from these rules; these are hazardous future inputs, not a claim that existing files were changed.

Validation: ran 19 temporary source probes through actual Rust `-a` and checked resulting syntax with Ruby 3.4.9; compared each first-pass rewrite with Wistia's pinned Ruby cops via Commissioner. Additional runtime probes verified changed arguments and exceptions. Artifacts: `/tmp/rr-review-correctors-probes.json`, `/tmp/rr-review-correctors-compared.json`, `/tmp/rr-review-correctors-runtime.json`.

## P1: Both table-option correctors append outside call parentheses

Location: `src/cop/ported/wistia_calls/migrations.rs:18` (through21).

```ruby
# Before: valid Rails migration call
create_table(:widgets)
# Rust -a after: invalid Ruby
create_table(:widgets), charset: "utf8mb4"
```

The first invalid pass stops subsequent useful correction; collation is not added. Selecting collation alone produces the same failure with `collation:`. Also reproduced with `create_table(:widgets) do |t| ... end`, assignment, and chained `.tap` calls. Ruby originals have the identical unsafe append algorithm. This is inherited behavior, not a Rust-only regression. Recommendation: insert keywords inside the actual argument list, before closing `)`, or decline autocorrection when no verified insertion point exists. Add actual CLI syntax-checked tests for both cops together and each separately.

## P1: NoSetex reorders argument evaluation and changes stored data / TTL

Location: `src/cop/ported/retained/simple.rs:22` (through27).

```ruby
events = [60, 1234]
redis.setex(key, events.shift, events.shift)
# becomes
redis.set(key, events.shift, ex: events.shift)
```

Before, the TTL is60 and value 1234; after, TTL 1234 and value 60. Confirmed by executing before/after with matching `setex(key, ttl, value)` and `set(key, value, ex:)` probe methods. Ordinary calls, assignments, callbacks, or TTL expressions that depend on mutable state can have the same issue. The Ruby original does the same. Recommendation: do not mark this transform safe for arbitrary expressions; preserve evaluation order or leave complex cases diagnostic-only. Simply setting unsafe only protects `-a`, so `-A` still needs guarding.

## P1: NoSetex swaps heredoc openers without moving their bodies

Location: same `simple.rs:23`–27.

```ruby
redis.setex(key, <<~TTL, <<~VALUE)
  60
TTL
  payload
VALUE
```

Rust produces:

```ruby
redis.set(key, <<~VALUE, ex: <<~TTL)
  60
TTL
  payload
VALUE
```

The bodies remain in original order. Ruby 3.4.9 rejects the corrected file because the second heredoc terminator is no longer found in order. A single value heredoc and a single TTL heredoc both passed; the combination fails. The pinned Ruby original produces the identical invalid output. Recommendation: refuse this correction when swapping multiple external heredocs, or reorder complete opener/body/terminator units with syntax-checked regression tests.

## P1: NoSetex turns a valid splatted call into invalid keyword syntax

Location: same `simple.rs:18`–27.

```ruby
ttl_and_value = [60]
redis.setex(key, *ttl_and_value, 1234)
# becomes
redis.set(key, 1234, ex: *ttl_and_value)
```

Original call expands to the required 3 arguments. The result is invalid Ruby (`ex: *...`). The pinned Ruby original does the same. NoSetex checks only that source arguments1 and2 exist; it does not validate static arity or argument shapes. Recommendation: refuse splats / keyword splats / uncertain arity before advertising a correction.

## P2: NoSetex loses a positional hash value supplied without braces

Location: same `simple.rs:18`–27; Rust treats each tree-sitter keyword pair as a separate argument.

```ruby
redis.setex(key, 60, a: 1, b: 2)
# Rust after
redis.set(key, a: 1, ex: 60, b: 2)
```

The original supplies a hash as the third positional argument to `setex(key, ttl, value)` in Ruby 3.4. The correction supplies only `key` positionally to `set(key, value, ex:)`; the runtime probe raises `ArgumentError`. Pinned Ruby groups the hash correctly but copies its source without braces, producing `redis.set(key, a: 1, b: 2, ex: 60)`, which also raises. Thus the AST argument-grouping difference is Rust-specific, while the runtime break is shared. Explicit `{a:1,b:2}` values do not have this problem. Recommendation: normalize the complete logical third argument and preserve explicit positional hash braces, or decline correction for bare keyword/hash arguments.

## P2: Table cops overwrite charset/collation already supplied through keyword splats

Location: `src/cop/ported/wistia_calls/migrations.rs:13`–21.

```ruby
options = {charset: "ascii", collation: "ascii_bin"}
create_table :widgets, **options
# Rust after
create_table :widgets, **options, charset: "utf8mb4", collation: "utf8mb4_unicode_ci"
```

Runtime probe confirms that Ruby keyword precedence changes the table options from the explicitly supplied ascii settings to the hardcoded defaults. Both table cops only recognize direct symbol pairs, not keys supplied by a splat. Direct `charset: "ascii"` / `collation: "ascii_bin"` are accepted unchanged, so this is an inconsistent change in configured table semantics. Pinned Ruby behaves the same. Recommendation: do not autocorrect unknown keyword splats; treat known literal hashes explicitly or leave a manual finding.

## Non-findings / boundaries

- Normal bare `create_table :widgets` receives both options across repeated passes and remains valid; no infinite correction loop observed.
- Direct `charset: nil, collation: nil` is accepted unchanged by both implementations; this is a policy hole if explicit concrete encodings are required, but it is not an autocorrection break.
- Single-heredoc NoSetex calls, ordinary 3-argument literal/name calls, and a trailing `&block` remained syntactically valid.
- Root separately confirmed a Rust-only engine bug where disable comments remove diagnostics but leave edits active; this report does not duplicate that finding.

Recommended immediate exposure reduction: disable these 3 custom autocorrectors pending correction-shape guards, preserving diagnostics. All findings above are reachable under today's rule settings; parity with the Ruby originals does not establish correction safety.

## Additional engine-accounting finding (P2, Rust-only)

`Cops/NoSetex` can modify a file while returning an empty successful JSON report (`offense_count:0`, `corrected_count:0`). Confirmed with a real temporary file and the current Wistia config:

```ruby
# before
redis.setex(key, events.shift, events.shift)
# file after -a
redis.set(key, events.shift, ex: events.shift)
```

The NoSetex diagnostic begins at the receiver (`n.start_byte()` at `simple.rs:14`), while its earliest edit begins at the method selector (`simple.rs:22`). `src/correction.rs:86`–106 reconciles `corrected` by whether the accepted edit covers the diagnostic's start, so this correctly applied edit loses its credit. The autocorrect loop only keeps credited corrected diagnostics; the final pass has no NoSetex finding left. The resulting report hides that the file changed. Fix correction ownership tracking so every accepted edit credits its parent diagnostic regardless of whether their byte ranges overlap; preserve transactional grouping for multiple edits. This is separate from the root's disable-comment bug.
