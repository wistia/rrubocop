URI.regexp("http")
# rrubocop-expect: 1:4 Lint/UriRegexp: `URI.regexp("http")` is obsolete and should not be used. Instead, use `URI::DEFAULT_PARSER.make_regexp("http")`.
