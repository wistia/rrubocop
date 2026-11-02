use super::super::super::context::*;
use tree_sitter::Node;
pub(super) fn key_content(c: &Context<'_>, key: Node<'_>) -> String {
    match key.kind() {
        "symbol_array" => percent_content(c, key, true),
        "string_array" => percent_content(c, key, false),
        "array" => kids(key)
            .iter()
            .map(|a| c.text(*a))
            .collect::<Vec<_>>()
            .join(", "),
        _ if literal(key) || key.kind() == "character" => c.text(key).into(),
        _ => format!("*{}", c.text(key)),
    }
}
fn percent_content(c: &Context<'_>, key: Node<'_>, symbol: bool) -> String {
    kids(key)
        .iter()
        .map(|a| percent_key(c, *a, symbol))
        .collect::<Vec<_>>()
        .join(", ")
}
fn percent_key(c: &Context<'_>, n: Node<'_>, symbol: bool) -> String {
    match (symbol, descendants(n).any(|d| d.kind() == "interpolation")) {
        (true, true) => format!(":\"{}\"", c.text(n)),
        (true, false) => format!(":{}", c.text(n)),
        (false, true) => format!("\"{}\"", c.text(n)),
        (false, false) => format!("'{}'", c.text(n)),
    }
}
