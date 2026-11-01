def m
 value = nil
 $reader = -> { value }
 value = 1
 ^ Style/RedundantAssignment: Redundant assignment before returning detected.
 value
end
p m
p $reader.call
