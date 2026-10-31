$redis.pipelined do |pipe|
 pipe.set("key0", "value")
 ^ Cops/NoSetWithoutExpiration: `$redis.set` must be called with an expiration option set (ex, exat, px, or pxat).
 pipe.set("key1", "value")
 ^ Cops/NoSetWithoutExpiration: `$redis.set` must be called with an expiration option set (ex, exat, px, or pxat).
end
