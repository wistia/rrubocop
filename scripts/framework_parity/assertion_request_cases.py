"""Original implicit assertion and request-alias roles with both supported styles."""
def extend(add):
    import json
    from pathlib import Path
    refute=("refute", "refute_empty", "refute_equal", "refute_in_delta", "refute_in_epsilon", "refute_includes", "refute_instance_of", "refute_kind_of", "refute_nil", "refute_operator", "refute_predicate", "refute_respond_to", "refute_same", "refute_match")
    preferred=("assert_not", "assert_not_empty", "assert_not_equal", "assert_not_in_delta", "assert_not_in_epsilon", "assert_not_includes", "assert_not_instance_of", "assert_not_kind_of", "assert_not_nil", "assert_not_operator", "assert_not_predicate", "assert_not_respond_to", "assert_not_same", "assert_no_match")
    for style in ("assert_not", "refute"):
        for row in json.loads((Path(__file__).parents[2]/"tests/fixtures/framework-refute-original-spec.json").read_text())["cases"]:
            add("Rails/RefuteMethods",row["source"],EnforcedStyle=style)
        for method in refute+preferred:
            for source in (f"{method} false", f"{method}(false)", f"self.{method}(false)", f"other.{method}(false)", f"{method}", f"{method}=1; {method}"):
                add("Rails/RefuteMethods",source,EnforcedStyle=style)
    for style in ("referer", "referrer"):
        for row in json.loads((Path(__file__).parents[2]/"tests/fixtures/framework-request-original-spec.json").read_text())["cases"]:
            add("Rails/RequestReferer",row["source"],EnforcedStyle=style)
        for receiver in ("request","request()","self.request","other.request","request(value)","request = value; request","request { work }","request&.call"):
            for suffix in (".referer", ".referrer", "&.referer", "&.referrer", ".referer()", ".referrer(1)", "::referrer", ".referrer { work }"):
                add("Rails/RequestReferer",receiver+suffix,EnforcedStyle=style)
    for source in ("def refute; end", "def self.refute; end", "alias refute previous", "undef refute", "refute false # rubocop:disable Rails/RefuteMethods", "refute(true,'custom')", "refute(true,nil)", "refute false, message", "assert_not = 123; refute"):
        add("Rails/RefuteMethods",source)
    add("Rails/RefuteMethods","refute false",AutoCorrect="disabled")
    add("Rails/RequestReferer","request.referrer",AutoCorrect="disabled")
    add("Rails/RequestReferer","request.referrer # rubocop:disable Rails/RequestReferer")
    for source in ("assert_not=1; refute false", "refute=1; assert_not false", "def f(refute); refute; end", "request.\nreferrer", "request. # keep\nreferrer", "request( # keep\n).referrer", "request.referrer { # keep\nwork }", "request.referrer = value"):
        add("Rails/RefuteMethods" if "refute" in source else "Rails/RequestReferer",source)
