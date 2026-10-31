use super::super::context::*;
use tree_sitter::Node;
pub(super) fn boolean_symbol(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let value = c.value(n);
    if !["true", "false"].contains(&value) || in_symbol_array(n) {
        return vec![];
    }
    let finding = Finding::new(
        n,
        format!("Symbol with a boolean name - you probably meant to use `{value}`."),
    );
    vec![correct_symbol(finding, n, value)]
}
fn in_symbol_array(n: Node<'_>) -> bool {
    ancestors(n).iter().any(|p| p.kind() == "symbol_array")
}
fn correct_symbol(finding: Finding, n: Node<'_>, value: &str) -> Finding {
    if n.kind() == "hash_key_symbol" {
        finding.edit(n.start_byte(), n.end_byte() + 1, format!("{value} =>"))
    } else {
        finding.replace(n, value)
    }
}
