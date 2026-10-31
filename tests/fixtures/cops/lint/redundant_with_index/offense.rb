items.each_with_index { |item| use(item) }
# rrubocop-expect: 1:6 Lint/RedundantWithIndex: Use `each` instead of `each_with_index`.
