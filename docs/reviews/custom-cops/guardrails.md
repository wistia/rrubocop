# Read-only review: 19 diagnostic-only app-specific ports

Reviewed commit `586bb4c` in `/home/ubuntu/code/rrubocop-wistia-parity` against Wistia's pinned RuboCop 1.65.1, Ruby 3.4.9, and AnyCable Rails Core 1.6.2.

## Autocorrection conclusion

**None of these 19 cops rewrites source.** All register with `correctable:false`, construct only `Finding::new`/`Finding::range` without edits, and remain diagnostic-only even with `-A`. Runtime probes executed a positive fixture for every cop against a temporary project: all 19 produced their expected diagnostic, every corrected count was zero, and every file remained byte-identical. Thus these 19 additions introduce no direct source-damage risk during autocorrection. Their material risk is missed guardrails or false lint failures.

Evidence: `/tmp/rr-review-guardrails-noedit.json` (19/19 positive controls); `/tmp/rr-review-guardrails-probes.json` (Ruby-vs-Rust regression probes). Repositories were not modified.

## Confirmed Rust regressions

### P2: Redis transaction guard silently misses do/end bodies

Location: `src/cop/ported/wistia_runtime/redis.rs:26` (specifically the block_body-only check at28).

```ruby
$redis.pipelined do |pipe|
  pipe.set("key0", "value")
  pipe.set("key1", "value")
end
```

Pinned Ruby:2 `Cops/NoSetWithoutExpiration` offenses. Rust:0. Changing the delimiters to `{ ... }` makes Rust find both. Tree-sitter do/end body uses `body_statement`, while the Rust receiver inference requires `block_body`. These writes can create Redis keys without expiration. Current configuration enables this cop globally, so this is reachable today. No edits are applied with either result. Accept both body kinds and add paired do/end and brace tests. The original Ruby cop additionally misses a single-statement transactional body; that weakness is shared, not introduced by the port.

### P2: A comment bypasses the protected-write callback guard

Location: `src/cop/ported/wistia_calls/protected_writes.rs:22`.

```ruby
record.update_column(
  # explain the update
  :protected, true)
```

Pinned Ruby:1 `Cops/NoCallbackSkippingProtectedWrites` offense. Rust:0. Rust's shared `args` returns named comment nodes, so `arguments.len()==2` fails. The non-commented call flags correctly. This permits the exact callback-skipping protected write that can leave the media-file S3 private tag stale. Enabled on production Ruby outside `spec/` and `db/`; reachable today. Filter comments before validating positional arguments and add a multiline regression. There is no correction or source change.

### P2: AnyCable custom coder guard can be bypassed by an argument comment

Location: `src/cop/ported/wistia_runtime/anycable.rs:54`.

```ruby
stream_from(
  # channel name
  "abc", coder: JSON)
```

Pinned AnyCable cop:1 `AnyCable/StreamFrom` offense. Rust:0. `valid_stream_arguments` treats the comment as the first argument instead of the string. A custom coder unsupported by AnyCable passes lint. Current AnyCable channel config enables this; reachable in `app/channels/**/*.rb`. Filter comments before examining arguments. No edits occur.

### P2: Valid parenthesized Rails error reporting is rejected

Location: `src/cop/ported/wistia_runtime/rescue.rs:38`.

```ruby
begin
  work
rescue
  Rails.error().report(error)
end
```

Pinned Ruby:0 offenses. Rust:1 `Cops/RescueMustReportError`. Rust compares receiver source to literal `Rails.error`, while Ruby matches the AST call. Parentheses do not change this reporting behavior. Current configuration enables this cop on application code. Match nested call/constant nodes rather than source spelling. No autocorrection is offered, so this blocks lint but cannot break source automatically.

### P3: A local named raise counts as re-raising

Location: `src/cop/ported/wistia_runtime/rescue.rs:38`.

```ruby
begin
  work
rescue
  raise = false
  puts raise
end
```

Pinned Ruby:1 missing-report offense. Rust:0. Any identifier named `raise` is treated as a call, including the assignment target and local-variable read. This can hide an unreported exception; the spelling is unusual, so lower priority than the guardrail bypasses above. Current config permits this path and enables the cop. Distinguish actual sends and unresolved bare-call identifiers from local bindings (as the NoSleep implementation already does). No source changes occur.

## Reviewed inventory

- Cops/NoCSVGenerate
- Cops/NoCallbackSkippingProtectedWrites
- Cops/NoContentIndexCallsInBackfills
- Cops/NoDirectLanternConfigBuilderMutators
- Cops/NoForeignKeys
- Cops/NoSleep
- Cops/NoSidekiqReceive
- Cops/NoRequireOnAutoLoadedPaths
- Cops/NoSetWithoutExpiration
- Cops/RescueMustReportError
- Cops/EnforceNoShowInPolicies
- Cops/EnforceScopeInPolicies
- Cops/ExplicitSidekiqQueue
- Cops/NoConstantDefinitionsInSpecs
- Cops/NoTopLevelSideEffectsInAutoloadedFiles
- Custom/RequireRespondToAny
- AnyCable/InstanceVars
- AnyCable/StreamFrom
- AnyCable/PeriodicalTimers

Source checks covered receiver handling, scope boundaries, registration/correctability, path exclusions, and inherited helpers. Ruby's original `Custom/RequireRespondToAny` crashes on a single `format.any` body in the direct commissioner probe (`NoMethodError` on Symbol#receiver); Rust correctly leaves that body clean. That is an upstream weakness, not a new Rust regression.

### P2: AnyCable reports nested target-construction options as stream coder options

Location: `src/cop/ported/wistia_runtime/anycable.rs:28`.

```ruby
stream_for(record.with_options(coder: JSON))
```

Pinned AnyCable:0 offenses. Rust:1 unsupported-coder offense. Rust recursively searches every descendant of the stream call, so `coder:` in a nested target-building call is treated as a keyword passed to `stream_for`. Ruby restricts the check to top-level hash arguments, which is the correct boundary for stream options. Enabled for channel files today; this is a false lint failure, with no correction offered. Search top-level option hash pairs rather than all descendants. Additional probe is included in `/tmp/rr-review-guardrails-probes.json`.
