if /foo/; run; end
# rrubocop-expect: 1:3 Lint/RegexpAsCondition: Do not use regexp literal as a condition. The regexp literal matches `$_` implicitly.
