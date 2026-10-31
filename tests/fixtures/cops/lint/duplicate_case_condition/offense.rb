case x
when 1; a
when 1; b
end
# rrubocop-expect: 3:5 Lint/DuplicateCaseCondition: Duplicate `when` condition detected.
