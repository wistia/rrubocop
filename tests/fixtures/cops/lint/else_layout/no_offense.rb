if condition
  foo
else
  bar
end
if x
  y
elsif z
  w
end
# Comment on else line is not a body statement (peatio rack_attack)
if API_LIMITS_ENABLED
  apply_rules
else # backward compatibility
  module Rack
    class Attack
    end
  end
end
if a then b else c end
