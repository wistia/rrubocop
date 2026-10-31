%w(foo %w(bar))
# rrubocop-expect: 1:0 Lint/NestedPercentLiteral: Within percent literals, nested percent literals do not function and may be unwanted in the result.
