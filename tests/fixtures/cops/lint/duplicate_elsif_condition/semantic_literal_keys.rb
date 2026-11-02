if value == :ready
 :first
elsif value == :"\x72eady"
 :second
end
# rrubocop-expect: 3:6 Lint/DuplicateElsifCondition: Duplicate `elsif` condition detected.
