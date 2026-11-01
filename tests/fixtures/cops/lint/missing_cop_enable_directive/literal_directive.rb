value = <<~TEXT
# rubocop:disable Layout/TrailingWhitespace
TEXT
value = "first
# rubocop:disable Layout/TrailingWhitespace
last"
value = %q(first
# rubocop:disable Layout/TrailingWhitespace
last)
value = %r{first
# rubocop:disable Layout/TrailingWhitespace
last}
