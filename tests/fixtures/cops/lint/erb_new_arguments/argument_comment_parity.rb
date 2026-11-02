ERB.new(
# parity audit
str, nil, "-", "@buf")
# rrubocop-expect: 3:5 Lint/ErbNewArguments: Passing safe_level with the 2nd argument of `ERB.new` is deprecated. Do not use it, and specify other arguments as keyword arguments.
# rrubocop-expect: 3:10 Lint/ErbNewArguments: Passing trim_mode with the 3rd argument of `ERB.new` is deprecated. Use keyword argument like `ERB.new(str, trim_mode: "-")` instead.
# rrubocop-expect: 3:15 Lint/ErbNewArguments: Passing eoutvar with the 4th argument of `ERB.new` is deprecated. Use keyword argument like `ERB.new(str, eoutvar: "@buf")` instead.
