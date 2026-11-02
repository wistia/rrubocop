# Original AST string values

`ported::original_string_value` projects pinned RuboCop AST StrNode/DstrNode `value` without evaluating Ruby. Existing string/delimiter/escape and static-heredoc decoders remain authoritative. Static fragments decode normally; interpolation Begin nodes retain their complete `#{...}` source, while shorthand ivar/global children retain `@name`/`$name` without the interpolation marker. Chained and physical multiline strings concatenate their original fragment values. CRLF source-buffer normalization is preserved.

All 29 authored original observations pass, covering multiple/nested/short interpolations, escaping, Unicode, paired percent delimiters, chained strings, physical/escaped/continued newlines, characters and static plain/quoted/squiggly heredocs. The fixture refresh script parses authored programs and reads AST values; it never executes them. Regeneration is byte-identical and the generator passes default pinned RuboCop 1.65.1 lint.

Dynamic heredoc values and XstrNode's AST-object-valued `value` are deliberately unsupported by this string-returning API. It is not a general runtime string evaluator and callers must independently decide correction safety. Existing static literal APIs are unchanged.

The existing source-reviewed rational comparator decoder is also exposed at crate scope without algorithm changes so consumers can format its reduced numerator/denominator instead of adding another numeric parser.
