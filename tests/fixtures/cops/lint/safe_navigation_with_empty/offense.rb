return if foo&.empty?
# rrubocop-expect: 1:10 Lint/SafeNavigationWithEmpty: Avoid calling `empty?` with the safe navigation operator in conditionals.
