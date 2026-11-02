seen = []
str = "ab"
value = str.split("") { |character| seen << character }
p [seen, value]
# rrubocop-expect: 3:12 Style/StringChars: Use `chars` instead of `split("")`.
