Foo = Class.new(Object) do
 class Wrong; end
end
# rrubocop-expect: 2:1 Cops/NoUndiscoverableAutoLoadedConstants: This constant is undiscoverable by the auto loader due to where it is nested.
