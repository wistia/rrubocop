"""Pinned SafeNavigation matcher, policy and ActiveSupport semantic probes."""
def extend(add):
    import json
    from pathlib import Path
    for convert in (False,True):
        for ruby in (2.2,2.3,3.4):
            for row in json.loads((Path(__file__).parents[2] / "tests/fixtures/framework-safe-nav-original-spec.json").read_text())["cases"]:
                add("Rails/SafeNavigation",row["source"],ConvertTry=convert,TargetRubyVersion=ruby)
        for dispatch in (":foo", ":foo?", ":foo!", ":foo=", ":[]", ":==", ":+", ":'foo'", ':"foo"', ":'foo bar'", ":'é'", ":föö", ':""', "%s(foo)", ":@foo", ':"Foo::Bar"', ':"foo\nb"', ':"foo\\nb"', "name", "'foo'", "nil", "123"):
            for method in ("try", "try!"):
                add("Rails/SafeNavigation",f"foo.{method}({dispatch})",ConvertTry=convert)
        for receiver in ("foo", "nil", "self", "", "[1,2]", "'abc'", "{a: 1}", "-1", "+1", "- 1.5", "__LINE__"):
            for method in ("try", "try!"):
                prefix=receiver+'.' if receiver else ''
                add("Rails/SafeNavigation",f"{prefix}{method}(:to_s)",ConvertTry=convert)
    for source in ("nil.try!(:concat, effect())", "nil.try!(:foo, {key => 1})", 'nil.try!(:foo, "#{effect()}")', "nil.try!(:foo, 1)", "foo.try!(:bar, a: 1, b: 2)", "foo.try!(:bar, **options)", "foo.try!(:bar, &block)", "foo.try!(:foo=)", "foo.try!(:foo=, 1, 2)", "p foo.try!(:foo=, 1)", "foo.try!(:foo=, 1)", "foo&.try!(:foo)", "foo.try!", "foo.try!()", "foo.try!(:bar) { |x| x }", "foo.try!(:bar) { _1 }", "foo.try!( # keep\n:bar)", "foo.try!(:bar)\r\n", "puts +1.try!(:abs)", "-1.foo.try!(:bar)"):
        add("Rails/SafeNavigation",source,ConvertTry=True)
    add("Rails/SafeNavigation","foo.try!(:bar)",AutoCorrect="disabled")
    add("Rails/SafeNavigation","foo.try!(:bar) # rubocop:disable Rails/SafeNavigation")
    for source in ("foo.try!(:bar,\n 1)\np __LINE__", "foo.try!(:bar,\n 1)\nobj.__LINE__", "def __LINE__; 1; end\nfoo.try!(:bar,\n 1)", "name=:foo; foo.try!(name)", "foo.try! :bar", "foo.try! :bar, first: 1, last: 2", "foo::try!(:bar)", "'abc'.try(:upcase)", "foo.try!(:bar) { # keep\nwork }", "foo.try!(:bar, <<~TEXT)\nbody\nTEXT"):
        add("Rails/SafeNavigation",source,ConvertTry=True)
