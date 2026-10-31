ERB.new(str, nil, "-", "@buf")
# rrubocop-expect: 1:13 Lint/ErbNewArguments: Passing safe_level with the 2nd argument of `ERB.new` is deprecated. Do not use it, and specify other arguments as keyword arguments.
# rrubocop-expect: 1:18 Lint/ErbNewArguments: Passing trim_mode with the 3rd argument of `ERB.new` is deprecated. Use keyword argument like `ERB.new(str, trim_mode: "-")` instead.
# rrubocop-expect: 1:23 Lint/ErbNewArguments: Passing eoutvar with the 4th argument of `ERB.new` is deprecated. Use keyword argument like `ERB.new(str, eoutvar: "@buf")` instead.
