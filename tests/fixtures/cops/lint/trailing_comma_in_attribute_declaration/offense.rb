class A
 attr_reader :foo,
 def bar; 1; end
end
# rrubocop-expect: 2:17 Lint/TrailingCommaInAttributeDeclaration: Avoid leaving a trailing comma in attribute declarations.
