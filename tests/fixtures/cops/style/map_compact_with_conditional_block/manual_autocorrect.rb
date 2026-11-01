p [nil, 1].map { |x| x if true }.compact
# rrubocop-expect: 1:11 Style/MapCompactWithConditionalBlock: Replace `map { ... }.compact` with `select`.
