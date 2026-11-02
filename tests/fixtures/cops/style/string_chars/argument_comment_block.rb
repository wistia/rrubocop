str.split(
 # delimiter
 "") { |character| use(character) }
# rrubocop-expect: 1:4 Style/StringChars: Use `chars` instead of `split(\n # delimiter\n "")`.
