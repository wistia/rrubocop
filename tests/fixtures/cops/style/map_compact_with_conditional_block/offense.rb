items.map { |x| x if allowed?(x) }.compact
# rrubocop-expect: 1:6 Style/MapCompactWithConditionalBlock: Replace `map { ... }.compact` with `select`.
