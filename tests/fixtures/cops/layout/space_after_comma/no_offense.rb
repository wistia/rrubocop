puts "literal,a #{foo(1, 2)} b,c"
puts /literal,a #{foo(1, 2)} b,c/
puts %W[literal,a #{foo(1, 2)} b,c]
puts <<~TEXT
literal,a #{foo(1, 2)} b,c
TEXT
puts "\#{foo(1,2)}"
puts '#{foo(1,2)}'
puts <<'LITERAL'
#{foo(1,2)}
LITERAL
