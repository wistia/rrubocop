use super::super::context::*;
use tree_sitter::Node;
pub(super) fn boolean_symbol(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let Some(value) = c.static_symbol(n) else {
        return vec![];
    };
    if !["true", "false"].contains(&value.as_ref()) || in_symbol_array(n) {
        return vec![];
    }
    let finding = Finding::new(
        n,
        format!("Symbol with a boolean name - you probably meant to use `{value}`."),
    );
    vec![correct_symbol(c, finding, n, &value)]
}
fn in_symbol_array(n: Node<'_>) -> bool {
    ancestors(n).any(|p| p.kind() == "symbol_array")
}
fn correct_symbol(c: &Context<'_>, finding: Finding, n: Node<'_>, value: &str) -> Finding {
    if n.kind() == "hash_key_symbol" {
        finding.edit_with(c, n.start_byte(), n.end_byte() + 1, || {
            format!("{value} =>")
        })
    } else {
        match c.text(n).strip_prefix(':') {
            Some(source) => finding.replace_with(c, n, || source),
            None => finding,
        }
    }
}

#[cfg(test)]
mod tests;
