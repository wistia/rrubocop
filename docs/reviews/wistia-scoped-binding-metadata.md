# Scoped binding metadata

This checkpoint unions the Performance agent's byte-to-binding identity map (ba767c5) with Core's synthetic-read filter (fda2df9). Membership remains unchanged: reflective reads do not become syntax tokens. The existing one-shot cached scope model additionally records numbered binding IDs per block entry span (IDs are unordered); no new lexical scanner or model parse is added.

Public shared `ruby_expression::{local_binding,numbered_bindings}` APIs expose these identities. The numbered query rejects explicit parameters and maps lambda bodies to their actual model block. Callers must apply their original parser-version policy (numbered blocks start in Ruby2.7); the shared model represents current Ruby semantics.

Validation: binding-shadow identity and four block/lambda/field-role controls pass, and the full native suite passes 929 tests with one ignored. The active WithOptions differential matrix (102 original cases, 204 second passes and 15 trusted programs) exercises these APIs; all native programs preserve original behavior. This prerequisite does not itself claim another completed framework rule.
