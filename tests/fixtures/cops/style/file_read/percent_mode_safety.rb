p File.open("sample.txt", %q(r)) { |f| f.read }
# rrubocop-expect: 1:2 Style/FileRead: Use `File.read`.
