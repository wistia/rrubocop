"""Original Exit receiver child-role and grouped argument-count branches."""
def extend(add):
    import json
    from pathlib import Path
    for row in json.loads((Path(__file__).parents[2]/"tests/fixtures/framework-exit-original-spec.json").read_text())["cases"]:
        add("Rails/Exit",row["source"])
    for receiver in ("", "Kernel.", "Process.", "::Kernel.", "Other::Kernel.", "Object.", "self.", "Kernel().", "other.Kernel.", "other&.Process.", "(Kernel).", "(Kernel()).", "Kernel {}.", "Kernel(1)."):
        for name in ("exit", "exit!", "abort"):
            for args in ("", "()", "(0)", "(1,2)", "(a:1,b:2)", "(0,a:1)", "(&block)"):
                add("Rails/Exit",receiver+name+args)
    for source in ("exit = 1; exit", "def exit; end", "def self.exit; end", "alias exit old", "undef exit", "object&.exit", "Kernel&.exit", "def f(exit); exit; end", "Kernel.\nexit!", "exit # rubocop:disable Rails/Exit", "exit { work }", "exit { _1 }", "exit()", "exit += 1"):
        add("Rails/Exit",source)
