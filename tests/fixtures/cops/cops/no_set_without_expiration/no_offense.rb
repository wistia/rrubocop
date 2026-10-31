$redis.multi { |conn| conn.set("k", "v") }
