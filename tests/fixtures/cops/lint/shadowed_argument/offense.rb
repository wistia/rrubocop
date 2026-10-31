def do_something(foo)
  foo = 42
  ^ Lint/ShadowedArgument: Argument `foo` was shadowed by a local variable before it was used.
  puts foo
end
