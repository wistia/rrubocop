case value
when 1
 :first
when 0x1
 :second
end
# rrubocop-expect: 4:5 Lint/DuplicateCaseCondition: Duplicate `when` condition detected.
