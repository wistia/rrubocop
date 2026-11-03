"""Original Presence matcher and raw Ruby parent-sibling geometry."""
def extend(add):
    import json
    from pathlib import Path
    for row in json.loads((Path(__file__).parents[2] / "tests/fixtures/framework-presence-original-spec.json").read_text())["cases"]:
        add("Rails/Presence",row["source"])
    for predicate in ("present?", "blank?"):
        for negative in ("", "!", "not "):
            for branches in (("value","nil"),("nil","value"),("value","other"),("other","value"),("different","nil")):
                add("Rails/Presence",f"value='x'; {negative}value.{predicate} ? {branches[0]} : {branches[1]}")
    expression="value.present? ? value : nil"
    for wrapper in ("{}", "[{}]", "[1, {}]", "call({})", "call(1, {})", "value = {}", "@value = {}", "VALUE = {}", "value += {}", "return {}", "return 1, {}", "yield {}", "yield 1, {}", "super({})", "({})", "begin; {}; end", "def method; {}; end", "def self.method; {}; end", "class Model; {}; end", "class Model < Parent; {}; end", "module Model; {}; end", "class << self; {}; end", "proc {{ {} }}", "proc do; {}; end", "if flag; {}; end", "if flag; first; else; {}; end", "while flag; {}; end", "[{}].first"):
        add("Rails/Presence",wrapper.format(expression))
    for other in ("work(value)", "work value", "obj.work value", "work first, last", "value[0]", "value + 1", "value < 1", "(other)", "begin; other; end", "if flag; other; end", "other rescue nil", "work while flag", "work until flag"):
        add("Rails/Presence",f"if value.present?\nvalue\nelse\n{other}\nend")
    add("Rails/Presence",expression,AutoCorrect="disabled")
    expression="value.present? ? value : nil"
    for wrapper in ("unless flag; {}; end", "unless flag; {}; else; work; end", "unless flag; work; else; {}; end", "if flag; else; {}; end", "begin; work; rescue; {}; end", "begin; work; rescue Error; {}; end", "begin; work; rescue Error => error; {}; end", "begin; work; ensure; {}; end", "begin; ensure; {}; end", "def f; work; ensure; {}; end", "{{key: {}}}", "value && {}", "value + {}", "call({})", "super({})", "-> {{ {} }}"):
        add("Rails/Presence",wrapper.format(expression))
    for other in ("value<1", "value << 1", "value &1", "work +1", "work +1, other", "left and right", "left or right", "obj.value = 1", "obj.value=1"):
        add("Rails/Presence",f"if value.present?\nvalue\nelse\n{other}\nend")
    for receiver,mirror in (("-1","-1"),("+1","1"),("- 1","-1"),("-0","0"),("-1.5","-1.5"),("-1r","-1.0r"),("-1i","-1.0i"),("-1.5i","-1.5i"),("-1.foo","-1.foo"),("-0.foo","0.foo"),("-1.foo(2)","-1.foo(2)"),("-1.foo","1.foo"),("-__LINE__","-__LINE__")):
        add("Rails/Presence",f"{receiver}.present? ? {mirror} : nil")
        add("Rails/Presence",f"if {receiver}.present?\n{mirror}\nelse\nnil\nend")
    add("Rails/Presence","value='x'; value.present? ? value : nil",AutoCorrect="disabled")
    add("Rails/Presence","value='x'; value.present? ? value : nil # rubocop:disable Rails/Presence")
