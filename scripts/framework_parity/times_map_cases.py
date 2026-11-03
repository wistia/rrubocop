"""Exact original TimesMap block/csend/literal shapes and safe correction bounds."""
def extend(add):
    import json
    from pathlib import Path
    cop="Performance/TimesMap"
    for row in json.loads((Path(__file__).parents[2] / "tests/fixtures/framework-times-map-original-spec.json").read_text())["cases"]:
        add(cop,row["source"])
    for count in ("0","4","-1","- 1","+2","4.2","-1.5","1r","1i","nil","false","'4'",":four","[1]","{a: 1}","/a/","n","@n","__LINE__","__FILE__","__ENCODING__","(4)"):
        for inner in (".","&.","::"):
            for outer in (".","&."):
                for method in ("map","collect"):
                    add(cop,f"{count}{inner}times{outer}{method} {{ |i| i.to_s }}")
    for source in ("4.times.map", "4.times.map()", "4.times.map(&:to_s)", "4.times.map(&block)", "4.times.map(&nil)", "4.times.map(1) { |i| i }", "4.times(1).map { |i| i }", "4.times { |i| i }.map { |i| i }", "times.map { |i| i }", "times().map { |i| i }", "4.times.map { _1 }", "4.times.map() { |i| i }", "4.times.map do |i|\ni\nend", "4.times. # keep\nmap { |i| i }", "4.times\n.map { |i| i }\np __LINE__", "4.times.map { |i| # keep\ni }", "4.times.map { |i| i }\r\n", "4.times.map = value"):
        add(cop,source)
    add(cop,"4.times.map { |i| i }",AutoCorrect="disabled")
    add(cop,"4.times.map { |i| i } # rubocop:disable Performance/TimesMap")
    for source in ("puts +1.times.map { |i| i }", "puts +1.times.map do |i| i end", "puts=1; puts +1.times.map do |i| i end", "puts 1.times.map do |i| i end", "puts(1.times.map do |i| i end)", "obj.print +1.times.collect do |i| i end"):
        add(cop,source)
