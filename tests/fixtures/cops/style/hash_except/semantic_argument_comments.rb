h.reject { |key, value| [:a].include?(
 # key
 key) }
# rrubocop-expect: 1:2 Style/HashExcept: Use `except(:a)` instead.
