if x = 1
     ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
  do_something
end

do_something if x = 1
                  ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.

return unless group = deq
                    ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.

until (parent, base = File.split(path); parent == path)
                    ^ Lint/AssignmentInCondition: Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.
  x
end
