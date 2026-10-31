hash.reject { |k, v| k == :a }
# rrubocop-expect: 1:5 Style/HashExcept: Use `except(:a)` instead.
