begin; x; rescue 1, "a", Error; recover; end
# rrubocop-expect: 1:10 Lint/RescueType: Rescuing from `1, "a"` will raise a `TypeError` instead of catching the actual exception.
