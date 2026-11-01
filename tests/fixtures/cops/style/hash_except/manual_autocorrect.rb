$events = []
def keys; $events << :keys; [:a]; end
h = {a: 1, b: 2}
p h.reject { |k, v| keys.include?(k) }
p $events
# rrubocop-expect: 4:4 Style/HashExcept: Use `except(*keys)` instead.
