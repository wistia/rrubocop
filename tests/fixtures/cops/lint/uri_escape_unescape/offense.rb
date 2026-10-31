URI.escape("x")
# rrubocop-expect: 1:0 Lint/UriEscapeUnescape: `URI.escape` method is obsolete and should not be used. Instead, use `CGI.escape`, `URI.encode_www_form` or `URI.encode_www_form_component` depending on your specific use case.
