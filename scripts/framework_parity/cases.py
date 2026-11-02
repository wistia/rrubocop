"""Pinned original-cop branch specimens for the framework parity queue."""

def cases():
    rows = []
    def add(cop, source, **config):
        rows.append(dict(id=f"{cop}:{len(rows)}", cop=cop, path="app/models/example.rb", source=source + "\n", expected=[], corrected=source + "\n", config=config))
    for selector in ("select", "filter", "find_all", "reject"):
        for counter in ("count", "length", "size"):
            add("Performance/Count", f"items.{selector} {{ |item| item.active? }}.{counter}")
        add("Performance/Count", f"items.{selector}(&:active?).count")
        add("Performance/Count", f"items&.{selector} {{ |item| item.active? }}&.count")
    for source in ("items.select.count", "items.select(:name).count", "items.select { |item| item.active? }.count(1)", "items.select { |item| item.active? }.count { |item| item.ready? }", "items.reject { |item| log(item); item.active? }.count", "items.reject(&predicate).count", "items.reject { |item| # keep this\n item.active? }.count"):
        add("Performance/Count", source)
    for source in ("items.nil? || items.empty?", "items == nil || items.empty?", "nil == items || items.empty?", "!items || items.empty?", "items.nil? || !!items.empty?", "nil? || empty?", "!items.present?", "!present?", "not items.present?", "def blank?; if ready?; !present?; end; end", "items.nil? || other.empty?", "items.nil? && items.empty?", "items&.nil? || items&.empty?", "def blank?; !present?; end", "perform unless items.present?", "unless items.present?\n perform\nend", "unless items.present?\n perform\nelse\n other\nend"):
        add("Rails/Blank", source)
    for option, source in (("NilOrEmpty", "items.nil? || items.empty?"), ("NotPresent", "!items.present?"), ("UnlessPresent", "perform unless items.present?")):
        add("Rails/Blank", source, **{option:False})
    for source in ("[1, 2].count", "{a: 1}.count", "items.to_a.count", "items.to_a { |n| n }.count", "items&.to_h&.count", "Array[1].count", "Hash[:a, 1].count", "Array(items).count", "Hash(items).count", "items.count", "[1, 2].count(1)", "[1].count { |n| n.odd? }"):
        add("Performance/Size", source)
    for source in ("URI::Parser.new", "::URI::Parser.new", "URI::Parser.new(options)", "Other::URI::Parser.new", "URI::Parser&.new", "URI::Parser.new { foo }"):
        add("Performance/UriDefaultParser", source)
    for source in ("(1..10).include?(n)", "(1...10).member?(n)", "(1..10)&.include?(n)", "((1..10)).include?(n)", "range.include?(n)", "('a'..'z').include?(text)"):
        add("Performance/RangeInclude", source)
    for source in ("!items.blank?", "!blank?", "items && !items.empty?", "!items.nil? && !items.empty?", "!!items && !items.empty?", "items != nil && !items.empty?", "items && !other.empty?", "perform unless items.blank?", "unless items.blank?\n perform\nend", "items&.blank?", "!items&.blank?"):
        add("Rails/Present", source)
    for option, source in (("NotBlank", "!items.blank?"), ("NotNilAndNotEmpty", "items && !items.empty?"), ("UnlessBlank", "perform unless items.blank?")):
        add("Rails/Present", source, **{option: False})
    for source in ("input.html_safe", "input&.html_safe", "raw(input)", "safe_concat(input)", "output.safe_concat(input)", "object.raw(input)", "\"safe\".html_safe", "\"#{input}\".html_safe", "I18n.t(:name).html_safe", "raw(I18n.t(:name))", "raw(t(:name))", "I18n.translate(:x).html_safe", "object.t(:name).html_safe", "html_safe", "input.html_safe(extra)"):
        add("Rails/OutputSafety", source)
    for cop, source in (("Rails/Blank", "items.nil? || items.empty?"), ("Performance/Count", "items.select { |item| item.active? }.count"), ("Performance/Size", "[1].count"), ("Performance/UriDefaultParser", "URI::Parser.new"), ("Performance/RangeInclude", "(1..3).include?(n)"), ("Rails/Present", "!items.blank?")):
        add(cop, source, AutoCorrect="disabled")
    add("Performance/Count", "éléments.select { |élément| élément.active? }.count")
    for cop, source in (("Rails/Blank", "élèves.nil? || élèves.empty?"), ("Rails/Blank", "items.nil? ||\n items.empty?"), ("Rails/Present", "élèves && !élèves.empty?"), ("Rails/Present", "items &&\n !items.empty?")):
        add(cop, source)
    return rows
