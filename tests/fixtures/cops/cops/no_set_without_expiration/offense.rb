$redis.set("key", "value", :'\x65x' => 60)
^ Cops/NoSetWithoutExpiration: `$redis.set` must be called with an expiration option set (ex, exat, px, or pxat).
