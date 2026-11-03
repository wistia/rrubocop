# RedundantParentheses real-corpus roles

Thirteen native-only findings on Wistia 310b8e42e91 exposed four differences
between tree-sitter shapes and original Ruby RuboCop 1.65.1 AST roles:

- Attribute assignments and non-arithmetic binary operators are original sends;
  `ignore_syntax?` / `like_method_argument_parentheses?` permits their sole grouped
  argument. Indexed assignments also supply send ancestry for logical expressions.
- A chained call after a do block can have an entire adjacent invocation's
  parentheses represented as parenthesized_statements inside argument_list.
  Those are invocation delimiters, not a Ruby Begin node.
- `a..` followed by a newline and `(b)` is an original range with an endpoint.
  Tree-sitter can instead return an endless range followed by a separate group,
  without reporting a syntax error.

The native predicate now preserves those exemptions. The continuation guard is
bounded to an adjacent sibling following an actual open range (optionally as an
assignment RHS), across only whitespace/comments. A semicolon remains a statement
boundary. It does not rewrite source or recover general parser errors.

Nineteen independent original observations include setter/local/index assignment,
comparison/match/arithmetic operators, nested real call arguments, chained block
calls, both range operators and comment/blank-line/semicolon controls. All three
focused native tests pass, including the existing 324-state original corpus and
source-loss safeguard corpus. New diagnostic ranges/messages and corrected bytes
match original Ruby. The clarity-formatted reference generator regenerates the
fixture byte-identically.

Fresh actual Wistia configuration CLI across all 56 files containing the 146
native-only findings owned by these four core repairs reports 0 offenses, exit 0,
no stderr. These patches preserve original diagnostics and do not change cop
configuration or claim combined autocorrection scheduling parity.
