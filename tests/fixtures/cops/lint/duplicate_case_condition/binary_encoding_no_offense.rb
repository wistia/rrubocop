# encoding: ASCII-8BIT
value = "\u00E9"
case value
when "\xC3\xA9"
 puts "binary"
when "\u00E9"
 puts "unicode"
end
