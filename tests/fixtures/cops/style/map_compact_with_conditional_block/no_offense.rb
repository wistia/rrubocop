items.map { |x| if test(x); next x; else; nil; log(x); end }.compact
