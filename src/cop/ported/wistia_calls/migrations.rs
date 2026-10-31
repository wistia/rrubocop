use super::super::context::*;
use tree_sitter::Node;

pub(super) fn charset(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    table_option(c, n, "charset", "utf8mb4")
}

pub(super) fn collation(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    table_option(c, n, "collation", "utf8mb4_unicode_ci")
}

fn table_option(c: &Context<'_>, n: Node<'_>, key: &str, value: &str) -> Vec<Finding> {
    if c.method(n) != "create_table" || c.symbol_pair_value(n, key).is_some() {
        return vec![];
    }
    let end = call_end(c, n);
    let call = String::from_utf8_lossy(&c.source.as_bytes()[n.start_byte()..end]);
    vec![Finding::new(n, format!("Missing {key} option")).edit(
        n.start_byte(),
        end,
        format!("{call}, {key}: \"{value}\""),
    )]
}

fn call_end(c: &Context<'_>, n: Node<'_>) -> usize {
    let end = block(n).map(|b| b.start_byte()).unwrap_or(n.end_byte());
    c.source.as_bytes()[n.start_byte()..end]
        .iter()
        .rposition(|b| !b.is_ascii_whitespace())
        .map(|i| n.start_byte() + i + 1)
        .unwrap_or(end)
}
