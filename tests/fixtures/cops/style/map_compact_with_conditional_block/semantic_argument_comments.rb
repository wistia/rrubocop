items.map { |item| item if keep?(item) }.compact(
 # no arguments
)
# rrubocop-expect: 1:6 Style/MapCompactWithConditionalBlock: Replace `map { ... }.compact` with `select`.
