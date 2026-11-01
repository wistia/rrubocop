def foo; "x" # frozen_string_literal: true
         ^ Style/TrailingBodyOnMethodDefinition: Place the first line of a multi-line method definition's body on its own line.
end
p foo.frozen?
