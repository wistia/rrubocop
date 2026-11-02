p File.open("sample.txt", ?w) { |f| f.write("new") }
# rrubocop-expect: 1:2 Style/FileWrite: Use `File.write`.
