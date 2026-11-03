# Framework semantic model prerequisites

Imported the stable Core prerequisites 7c58ce9 → 12934f2 → fda2df9: exact `src/model/{masgn,mod,scopes,writes,reads,local_identifiers}.rs` from fda2df9, the matching synthetic reflective-read constructor, and only the three appended Context role tests. Existing framework Context/string/comparator code is preserved.

The model now records destructuring/for/header bindings, scoped implicit numbered block parameters (including outer-evaluated definition headers), and reflective reads separately from actual local-variable tokens. This prerequisite is needed for future receiver-removal guards; it does not itself claim another implemented framework rule.

Validation: 926 native tests pass, one ignored. Full framework differential regression completed 3,529 original cases, 7,058 second-pass cases and 87 trusted programs. Its 2,924 retained events exactly equal the previous 2,715 plus HttpStatus83, IndexBy/IndexWith120 and lexical-filter6; the model import introduced no additional prior-rule differences. Current model numbered-parameter classification follows current Ruby semantics; old parser targets need explicit version handling at rule adapters and remain a broader shared-role review item.

Performance integration preserves the declaration-byte map added in ba767c5 while retaining the synthetic-read filter; the role, declaration-shadowing and numbered-binding tests remain present.
