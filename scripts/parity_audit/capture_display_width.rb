# frozen_string_literal: true

require "bundler/setup"
require "unicode/display_width"
require "json"
output = ARGV.fetch(0)
width = Unicode::DisplayWidth
abort "Expected pinned unicode-display_width 2.5.0" unless width::VERSION == "2.5.0"
index = width.const_get(:INDEX)
first = width.const_get(:FIRST_4096)
intervals = []
(0..0x10ffff).each do |codepoint|
  next if (0xd800..0xdfff).cover?(codepoint)
  value = if codepoint > 15 && codepoint < 161
    1
  elsif codepoint < 0x1001
    first[codepoint]
  else
    current = index
    depth = 0x10000
    remaining = codepoint
    while (current = current[remaining / depth]).is_a?(Array)
      remaining %= depth
      depth /= 16
    end
    current
  end
  value = 1 if value.nil? || value == :A
  next if value == 1
  if intervals.last && intervals.last[1] + 1 == codepoint && intervals.last[2] == value
    intervals.last[1] = codepoint
  else
    intervals << [codepoint, codepoint, value]
  end
end
samples = ["", "abc", "Ｒｕｂｙ", "é", "e\u0301", "😀", "🇺🇸", "👩‍💻", "\t", "\b", "ab\b", "\b\babc", "\n", "\x05", "\u200b", "\u00ad", "\ufffd", "\u{10ffff}", "é\b"]
File.write(output, JSON.pretty_generate({reference: "unicode-display_width #{width::VERSION}", unicode: width::UNICODE_VERSION, intervals:, samples: samples.map { |source| {source:, width: width.of(source)} }}) + "\n")
