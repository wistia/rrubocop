items.each_with_object([]) { |item| use(item) }
# rrubocop-expect: 1:6 Lint/RedundantWithObject: Use `each` instead of `each_with_object`.
