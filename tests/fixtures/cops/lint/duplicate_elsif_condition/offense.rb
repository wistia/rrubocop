if x == 1; a; elsif x == 1; b; end
# rrubocop-expect: 1:20 Lint/DuplicateElsifCondition: Duplicate `elsif` condition detected.
