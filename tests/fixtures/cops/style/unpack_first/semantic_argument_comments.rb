str.unpack("C*").first(
 # no arguments
)
# rrubocop-expect: 1:4 Style/UnpackFirst: Use `unpack1("C*")` instead of `unpack("C*").first(\n # no arguments\n)`.
