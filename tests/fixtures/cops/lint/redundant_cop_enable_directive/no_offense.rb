# rubocop:disable Style/StringLiterals
x = "a"
# rubocop:enable Style/StringLiterals

value = <<~TEXT
# rubocop:disable Layout/TrailingWhitespace, Layout/TrailingWhitespace
# rubocop:enable Layout/TrailingWhitespace
TEXT
value = "first
# rubocop:disable Layout/TrailingWhitespace, Layout/TrailingWhitespace
# rubocop:enable Layout/TrailingWhitespace
last"
value = %q(first
# rubocop:disable Layout/TrailingWhitespace, Layout/TrailingWhitespace
# rubocop:enable Layout/TrailingWhitespace
last)
value = %r{first
# rubocop:disable Layout/TrailingWhitespace, Layout/TrailingWhitespace
# rubocop:enable Layout/TrailingWhitespace
last}
