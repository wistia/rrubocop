redis.setex(key, ttl, value)
^ Cops/NoSetex: `setex(key, expires_in, value)` is deprecated, please use `set(key, value, ex: expires_in)` instead
