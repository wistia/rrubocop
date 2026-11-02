case x
when %s|a|, :a; run
end
# rrubocop-expect: 2:12 Lint/DuplicateCaseCondition: Duplicate `when` condition detected.
