"""Pinned Rails RelativeDateConstant branch probes."""
def extend(add):
    import json
    from pathlib import Path
    for row in json.loads((Path(__file__).parents[2] / "tests/fixtures/framework-relative-date-original-spec.json").read_text())["cases"]:
        add("Rails/RelativeDateConstant", row["source"].rstrip("\n"))
    for source in ("DATE = Date.yesterday\np DATE", "DATE = clock.ago", "DATE = Date.yesterday\np Object.const_get(:DATE)", "DATE = Date.yesterday\np Object.constants", "p(DATE = Date.yesterday)", "DATE = [Date.yesterday, other.ago]", "DATE = if flag; ago; else; tomorrow; end", "DATE = Date.yesterday # keep", "DATE = Time.yesterday", "DATE = 1.yesterday", "DATE = Time.now.ago", "DATE = __LINE__.days.ago", "DATE = Date.yesterday\np __LINE__", "class Example\n  DATE = Date.yesterday\nend", "DATE = Time.now.yesterday", "DATE = 1.day.ago.to_date", "DATE = ago if flag", "DATE = proc { _1; ago }", "DATE = proc { _1; ago }", "DATE, OTHER = [ago, tomorrow]"):
        add("Rails/RelativeDateConstant", source)
    for ruby in (2.5,2.7,3.4):
        add("Rails/RelativeDateConstant", "DATE = proc { _1; ago }", TargetRubyVersion=ruby)

    for source in ("DATE = 1e999.days.ago", "DATE = 999999999999999999999999999.years.ago", "DATE = Date.yesterday\nOTHER = 2", "DATE = Date.yesterday; DATE", "DATE = ago; OTHER = tomorrow", "A::DATE ||= ago", "DATE, OTHER = ago, tomorrow", "DATE = 1.day.ago # rubocop:disable Rails/RelativeDateConstant"):
        add("Rails/RelativeDateConstant", source)
    for source in ("class Example; NEW = Date.yesterday; end", "class Example; NAME = Date.yesterday; end", "class Example < Parent; DATE = Date.yesterday; end"):
        add("Rails/RelativeDateConstant", source)
    for source in ("DATE ||= ago { side }", "DATE ||= ago { _1 }", "DATE, OTHER = ago, # keep\ntomorrow", "DATE, # keep\nOTHER = ago, tomorrow", "DATE = tomorrow.ago", "DATE = ago.tomorrow", "OUTER = (INNER = ago)"):
        add("Rails/RelativeDateConstant", source)
    for source in ("DATE = <<~SQL\n#{Date.yesterday}\nSQL", "DATE = <<~'SQL'\n#{Date.yesterday}\nSQL", "DATE = <<~SQL\n#{proc { Date.yesterday }}\nSQL", "DATE = <<~SQL\n#{proc { _1; Date.yesterday }}\nSQL", "DATE = [<<~SQL, <<~OTHER]\n#{Date.yesterday}\nSQL\n#{1.day.ago}\nOTHER", "DATE ||= <<~SQL\n#{Date.yesterday}\nSQL"):
        add("Rails/RelativeDateConstant", source)
    methods = ("since", "from_now", "after", "ago", "until", "before", "yesterday", "tomorrow", "today")
    for method in methods:
        for value in (f"1.day.{method}", f"clock.{method}", f"clock.{method}(base)", f"clock&.{method}", f"{method}()", f"-> {{ clock.{method} }}", f"proc {{ clock.{method} }}", f"proc {{ _1; clock.{method} }}", f"[clock.{method}]", f"{{date: clock.{method}}}"):
            add("Rails/RelativeDateConstant", f"class Example\n  DATE = {value}\nend")
        for assignment in (f"DATE ||= clock.{method}", f"Other::DATE = clock.{method}", f"DATE, OTHER = clock.{method}, 1", f"DATE, OTHER = [clock.{method}, 1]", f"DATE ||= wrapper(clock.{method})"):
            add("Rails/RelativeDateConstant", assignment)
    for value in ("Proc.new { 1.year.ago }", "1.day.ago..Date.current", "Date.current...1.day.since", "1.day.ago.tomorrow", "ago = 1; ago", "obj.ago = 1", "foo(ago)", "proc { _1; tomorrow }"):
        add("Rails/RelativeDateConstant", f"DATE = {value}")
    add("Rails/RelativeDateConstant", "DATE = Date.yesterday", AutoCorrect="disabled")
    add("Rails/RelativeDateConstant", "DATE = Date.yesterday # rubocop:disable Rails/RelativeDateConstant")
