# Framework semantic model prerequisites

Imported the stable Core prerequisites 7c58ce9 → 12934f2 → fda2df9: `src/model/{masgn,mod,scopes,writes,reads,local_identifiers}.rs` behaviour from fda2df9, the matching synthetic reflective-read constructor, and only the three appended Context role tests. Existing framework Context/string/comparator code is preserved.

The model now records destructuring/for/header bindings, scoped implicit numbered block parameters (including outer-evaluated definition headers), and reflective reads separately from actual local-variable tokens. This prerequisite is needed for future receiver-removal guards; it does not itself claim another implemented framework rule.

Validation at checkpoint: 926 native tests pass, one ignored. Full framework differential regression is running; results will be recorded in the next implementation checkpoint. Current model numbered-parameter classification follows current Ruby semantics; old parser targets need explicit version handling at rule adapters and remain a broader shared-role review item.

Performance integration preserves the declaration-byte map added in ba767c5 while retaining the new synthetic-read filter; all three role tests and the declaration-shadowing test remain present.
