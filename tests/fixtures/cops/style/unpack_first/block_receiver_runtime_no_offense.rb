seen = []
value = begin
 "ab".unpack("C*") { |number| seen << number }.first
rescue NoMethodError
 :missing
end
p [seen, value]
