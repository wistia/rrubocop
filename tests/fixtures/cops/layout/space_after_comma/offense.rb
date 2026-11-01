puts "#{foo(1,2)}"
             ^ Layout/SpaceAfterComma: Space missing after comma.
puts /#{foo(1,2)}/
             ^ Layout/SpaceAfterComma: Space missing after comma.
puts %W[#{foo(1,2)}]
               ^ Layout/SpaceAfterComma: Space missing after comma.
puts <<~TEXT
#{foo(1,2)}
       ^ Layout/SpaceAfterComma: Space missing after comma.
TEXT
