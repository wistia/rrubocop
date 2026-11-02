use super::super::context::*;
use tree_sitter::Node;
mod insertion;

pub(super) fn charset(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    table_option(c, n, "charset", "utf8mb4")
}

pub(super) fn collation(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    table_option(c, n, "collation", "utf8mb4_unicode_ci")
}

fn table_option(c: &Context<'_>, n: Node<'_>, key: &str, value: &str) -> Vec<Finding> {
    if c.method(n) != "create_table" || has_option(c, n, key) {
        return vec![];
    }
    let f = Finding::send(n, format!("Missing {key} option"));
    vec![if let Some((position, comma)) = insertion::position(c, n) {
        f.edit_with(c, position, position, || {
            format!("{comma}{key}: \"{value}\"")
        })
    } else {
        f
    }]
}

fn has_option(c: &Context<'_>, n: Node<'_>, name: &str) -> bool {
    option_pairs(n)
        .filter_map(|pair| field(pair, "key"))
        .any(|key| c.static_symbol(key).is_some_and(|value| value == name))
}

#[cfg(test)]
mod tests;
