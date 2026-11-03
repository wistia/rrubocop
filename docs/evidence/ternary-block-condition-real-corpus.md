# Ternary block conditions

Ten native-only Style/TernaryParentheses findings on Wistia 310b8e42e91
were parenthesized block-call conditions under Standard's
require_parentheses_when_complex style. Ruby RuboCop 1.65.1 represents these
as block/numblock nodes; `complex_condition?` and `non_complex_expression?`
classify them as complex. Tree-sitter attaches the block to a call node, and
the native call branch incorrectly classified them as ordinary simple sends.

The native predicate now preserves the block distinction before classifying
plain call names. The fresh original corpus contains 24 independent states:
all three enforced styles, explicit and numbered blocks, safe navigation,
brace/do blocks, plain-call and parenthesized-argument controls. All three
focused native tests pass, including the existing original-spec corpus and
unsafe omission safeguard. Messages, ranges and correction bytes agree in
all 24 new states. No configuration or correction scheduling changes.
