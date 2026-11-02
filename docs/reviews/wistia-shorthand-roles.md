# Canonical omitted-value roles

The shared `ruby_expression::shorthand_value` adapter follows pinned parser3.3.5.0 `Parser::Builders::Default#pair_label` (lines521–532): first Unicode lowercase character produces an identifier resolved by existing scoped local membership; other labels produce a Const node. The public result is Constant, Local or Send. `regular_send_receiver` exposes the corresponding ordinary-send receiver projection, excludes safe sends and pseudo literals, and includes unbound omitted lowercase values.

Core supplied45 original cases/48 roles (44Const, twoLocal including a lowercase Unicode control, twoSend). The committed native test checks name/byte/type/local/purity observations and ordinary-send inclusion; all48 pass. This describes the original AST, not actual VM purity: an unbound underscore omitted value can invoke a Ruby method despite the pinned parser marking it Const/pure. Correction guards must separately preserve runtime behavior.

The traversal helper now explicitly preserves its input node lifetime so callers can collect borrowed nodes without another walker. No traversal behavior changes.
