if foo
  one
else bar
     ^^^ Lint/ElseLayout: Odd `else` layout detected. Did you mean to use `elsif`?
  baz
end
if baz
  two
else qux
     ^^^ Lint/ElseLayout: Odd `else` layout detected. Did you mean to use `elsif`?
  wibble
end
if something then test
else something_else
     ^^^^^^^^^^^^^^ Lint/ElseLayout: Odd `else` layout detected. Did you mean to use `elsif`?
  other
end
if something then test
else something_else
     ^^^^^^^^^^^^^^ Lint/ElseLayout: Odd `else` layout detected. Did you mean to use `elsif`?
end
