redis.setex("clé", 1, "é") { callback }
# rrubocop-expect: 1:0 Cops/NoSetex: `setex(key, expires_in, value)` is deprecated, please use `set(key, value, ex: expires_in)` instead
