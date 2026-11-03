# Numbered-parameter real-corpus false positives

The optimized integrated 091dc33 scan of Wistia 310b8e42e91 reported 122
Lint/UnderscorePrefixedVariableName offenses absent from original Ruby RuboCop
1.65.1. These are implicit numbered block parameters. The semantic model now
records those bindings for other consumers, but this cop treated their first
read as an explicit variable declaration.

Original source: rubocop-1.65.1/lib/rubocop/cop/lint/underscore_prefixed_variable_name.rb,
`after_leaving_scope`, plus VariableForce's explicit-reference/declaration roles.
This patch excludes only numbered bindings in scopes without explicit parameters
and without assignment writes. It preserves explicit `_1` parameters and locals
accepted by older parser versions. The regression fixture contains 33 valid
independent original observations across TargetRubyVersion 2.6, 2.7 and 3.4,
including ordinary underscore parameters/locals and nested blocks/lambdas.
The positive controls also repaired an existing diagnostic range bug: the full
variable name is highlighted, rather than one character.

All three focused native tests pass, including exact original diagnostic
messages, ranges and unchanged correction bytes for all 33 states. This is a
bounded real-corpus regression repair, not completion of this cop's full source
review: keyword-block configuration and all reflective-reference branches remain
outside this checkpoint. No shared model semantics or enforcement were changed.
