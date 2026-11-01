# Remaining custom call/serializer performance opportunities

Read-only audit of implementation `3f5fca0`, branch HEAD `5b95a2e`. No source edits, builds or timing runs. This deliberately excludes already-fixed repeated parsing, materialized descendant walks, eager correction payloads and missing finite call-name gates. Dynamic confirmation belongs to the profiling agent; static complexity below is not a measured corpus speedup.

## Scope

18 app-specific rules: `Cops/CreateTableMustIncludeCharset`, `Cops/CreateTableMustIncludeCollation`, `Cops/NoCSVGenerate`, `Cops/NoCallbackSkippingProtectedWrites`, `Cops/NoContentIndexCallsInBackfills`, `Cops/NoDirectLanternConfigBuilderMutators`, `Cops/NoForeignKeys`, `Cops/NoSleep`, `Cops/NoSidekiqReceive`, `AnyCable/PeriodicalTimers`, `Cops/NoSetWithoutExpiration`, `Custom/RequireRespondToAny`, `Serializers/AvoidCapitalizedKeys`, `Serializers/AvoidKeyAndBlock`, `Serializers/AvoidMethodAndBlock`, `Cops/NoSetex`, `Serializers/PreferAttributeBlockOverAttributeAndMethod`, `Serializers/PreferAttributeBlockOverAttributesAndMethods`. The other 10 custom rules are covered by the scopes/autoload auditor.

## 1. Index serializer method existence instead of rescanning per attribute

Sources:

- `src/cop/ported/retained/serializers.rs:37-44`: both PreferAttributeBlock rules find the owner, then walk the whole owner subtree for **each** attribute name. Plural declarations repeat that search for every argument.
- `src/cop/ported/wistia_runtime/serializers.rs:54-57`: AvoidMethodAndBlock walks the parent subtree for **each** attribute block.

Streaming iteration removed temporary subtree vectors, but it did not remove repeated traversal. With A attributes and T owner/parent nodes, absent or late method matches still require O(A*T) work. This can become quadratic when declarations and method count grow together. It also occurs on clean serializers whose attributes do not match methods.

Recommendation: one source-snapshot method index. For PreferAttributeBlock rules, key by nearest class/module/singleton-class node identity plus method name. For AvoidMethodAndBlock, retain its intentionally different parent-subtree semantics: index method byte positions by name and range-query the exact parent. Do **not** collapse both checks into nearest-class semantics.

Safeguards: preserve nearest scope ownership for the two Prefer rules, reopened classes as separate node owners, methods inside nested lexical constructs, exclusion of singleton_method nodes, symbol-only attributes, plural offense ordering, and the broader nested-parent search of AvoidMethodAndBlock. All three checks are diagnostic-only; no method movement/autocorrection should be introduced. Existing serializer no-edit regressions remain acceptance gates.

Suggested measurement: one class with 100/200/400/800 attributes and matching methods at the end; separate no-match clean control; run the three rules separately against same-size Syntax controls. Assess actual eligible serializer files before claiming application impact. Sent to profiler.

## 2. Make Redis expiration guard cheap for compliant transactions

Sources: `src/cop/ported/wistia_runtime/redis.rs:8-11,28-45,62-64`.

For `redis.multi do |pipe| ... pipe.set(..., ex: 60) ... end`, each set call first classifies its transaction even though it will later find an explicit expiration and emit nothing. Classification repeatedly:

1. Materializes all non-comment body children to answer whether there are at least two statements (`transaction_body`, line63).
2. Lowercases and searches the **whole enclosing transaction source text**, including all its set statements (`transaction`, line45).
3. Extracts the same first block parameter.

With K direct sets in an N-byte/body-sized transaction this is O(K*N) work and allocation, including clean files. This differs from the previously fixed NoSetex reparsing issue.

Recommendations in order:

- Move the unchanged `has_expiration(c,n)` check before receiver/transaction classification. It is pure and its true result currently guarantees no offense regardless of receiver. This preserves payload-hash-versus-option semantics by retaining the existing helper, and completely avoids transaction classification on valid explicit-expiration calls.
- Replace full `kids(body).len() >= 2` with an early-stopping non-comment named-child count. **Do not** replace it with raw `named_child_count >= 2`: a comment is a named child, while `kids` excludes comments, so a one-statement-plus-comment block would change behavior.
- Cache the remaining transaction qualification and parameter name per exact body/call node in the immutable source snapshot. Missing-expiration violations then classify each transaction once. Preserve immediate-body membership to avoid leaking a transaction receiver into nested/shadowed blocks, existing one-statement behavior, multi/pipelined and braces/do variants, and the exact current case normalization/whole-text predicate. Do not silently narrow whole-text matching to receiver-only matching during a performance patch.

This cop is diagnostic-only. No autocorrection or correctability tradeoff is needed. Suggested measurement: N compliant set calls inside one redis.multi block versus N missing-expiration calls, with Syntax controls and identical corpus/path selection; sent to profiler. Real transactions are generally small, so worst-case and aggregate effects must be reported separately.

## 3. Add finite bare-identifier routing where measured useful

Sources: `src/cop/ported/mod.rs:61-65,102-115`; `src/cop/walker.rs:99-102`.

NoSleep, NoForeignKeys and PeriodicalTimers additionally register for `identifier`, because a bare zero-argument Ruby method can be represented as an identifier. Every identifier invokes the corresponding callback; only `bare_call` later compares text with sleep/add_foreign_key/periodically. Existing finite method-name gates apply to calls, not identifiers.

A static interested-identifier-name gate could avoid irrelevant callback/config/context dispatch on identifier-heavy files. Keep the current lexical-local, parameter, assignment-slot and bare-call checks **after** routing, so a local named sleep is still ignored. This is a potential broad but likely small framework opportunity; measure before extending the Cop trait/walker API. Profiler was asked for a many-ordinary-identifiers experiment with matching parse/model activation controls.

## Lower-priority bounded work

- `src/cop/ported/context.rs:167-178` materializes args and option pairs even for first-match queries. The three serializer runtime checks may independently extract the same attribute call/options; charset and collation each scan the same create_table options and insertion feasibility. A short-circuit option iterator or small shared call summary can reduce allocations, but common calls contain very few options, so a typed mutex cache may cost more than rescanning. Benchmark representative option-heavy calls before adding caches. Migration insertion feasibility must still run in diagnostic mode to preserve correctable metadata; do not skip it simply because edits are off.
- `Serializers/AvoidCapitalizedKeys` allocates a lowercase string for every literal (`wistia_runtime/serializers.rs:23-24`). An ASCII fast path can avoid allocation for common keys; retain Unicode fallback to exactly preserve lowercasing behavior. Expected tiny average benefit; not worth a separate architecture change without an allocation profile.
- NoSetex now performs bounded per-call work plus the exact shared index lookup; its former O(N²) file reparsing is fixed. Additional cache mutex/Arc or message cloning savings are micro-optimizations, not a second confirmed large issue.
- NoContentIndexCallsInBackfills remains a semantic method-name wildcard: any method on either content index receiver can offend. Do not invent an incomplete method gate. The existing receiver equality check is bounded. Lantern/protected writes/CSV/foreign-key/sleep/timer/Sidekiq/respond_to guards all have finite names or narrow local/ancestor checks; no additional material per-rule algorithmic issue found here.

## Recommended order

First confirm serializer and Redis scaling dynamically; implement the expiration-first/early-child-count changes and exact indexes if justified. Then measure identifier routing. Keep the existing 2,054 reference cases and actual Off/Safe/All pipeline coverage as semantic gates. None of these static observations demonstrates a whole-codebase time saving yet.
