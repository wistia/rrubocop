"""Original UniqBeforePluck source/spec branch matrix."""
def extend(add):
    import json
    from pathlib import Path
    for style in ("conservative", "aggressive"):
        for row in json.loads((Path(__file__).parents[2] / "tests/fixtures/framework-uniq-pluck-original-spec.json").read_text())["cases"]:
            add("Rails/UniqBeforePluck", row["source"].rstrip("\n"), EnforcedStyle=style)
        for receiver in ("Model", "::Model", "Other::Model", "object::Model", "__ENCODING__", "__FILE__", "Model.scope", "instance.assoc", "items", "(Model)", "", "super", "self", "Model&.scope"):
            pluck=f"{receiver}.pluck(:name)" if receiver else "pluck(:name)"
            for suffix in (".uniq", ".uniq()", ".uniq(nil)", ".uniq(extra)", ".uniq(&:itself)", ".uniq { |v| v[0] }", ".uniq { _1[0] }", "&.uniq", "::uniq", ".uniq = input"):
                add("Rails/UniqBeforePluck", pluck+suffix, EnforcedStyle=style)
        for source in ("Model.pluck(:name)\n  .uniq", "Model.pluck(:name).\n uniq", "Model.pluck(:name) # keep\n .uniq", "Model.pluck(:name). # keep\n uniq", "Model.pluck(:name).uniq\np __LINE__", "Model.pluck(:name)\n.uniq\np __LINE__", "Model.pluck(:name).uniq.uniq", "Model.pluck(:name) { work }.uniq", "Model.pluck(:name) { _1 }.uniq", "Model.pluck.uniq", "pluck.uniq", "pluck = []; pluck.uniq", "Model.pluck(:name).uniq { |v| v }", "Model.pluck(:name).uniq { _1 }", "Model.pluck(:name).uniq # rubocop:disable Rails/UniqBeforePluck", "Model.pluck(:name).uniq\r\n"):
            add("Rails/UniqBeforePluck", source, EnforcedStyle=style)
    add("Rails/UniqBeforePluck", "Model.pluck(:name).uniq", AutoCorrect="disabled")
    for ruby in (2.5,2.7,3.4):
        add("Rails/UniqBeforePluck", "Model.pluck(:name).uniq { _1 }",TargetRubyVersion=ruby)
    for source in ("(Model.pluck(:name)).uniq", "Model.pluck(:name).uniq(&nil)", "Model.pluck(:name).uniq(...)", "Model.pluck(:name).uniq { |;local| _1 }", "Model.pluck(:name).uniq do |value|; value; end", "Model.pluck(:name).uniq do; _1; end", "Model.pluck(:name).uniq( # keep\n)", "Model.pluck(:name).uniq\n# keep", "Model.pluck(:name).uniq?", "Model.pluck(:name).uniq!"):
        add("Rails/UniqBeforePluck", source, EnforcedStyle="aggressive")
    add("Rails/UniqBeforePluck", "Model.pluck(:name).uniq",SafeAutoCorrect=True)
    for source in ("Model&.pluck(:name).uniq", "Model::pluck(:name).uniq", "Élève.pluck(:nom).uniq", "Model.pluck(:name).uniq { _2 }", "Model.pluck(:name)\n  ::uniq"):
        for style in ("conservative", "aggressive"):
            add("Rails/UniqBeforePluck",source,EnforcedStyle=style)
