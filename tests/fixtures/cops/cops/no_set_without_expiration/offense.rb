$redis.set("k", "v")
# rrubocop-expect: 1:0 Cops/NoSetWithoutExpiration: `$redis.set` must be called with an expiration option set (ex, exat, px, or pxat).
