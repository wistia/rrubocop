use super::super::context::*;
use tree_sitter::Node;

pub(super) fn protected_writes(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let found = match c.method(n) {
        "update_column" => protected_column(c, n),
        "update_columns" | "update_all" => protected_hash(c, n),
        _ => false,
    };
    if !found {
        return vec![];
    }
    vec![Finding::new(
        n,
        format!(
            "Don't write `protected` with `{}`; it skips the callback that syncs media files' S3 `private` tag. Use `update`/`save`.",
            c.method(n)
        ),
    )]
}

fn protected_column(c: &Context<'_>, n: Node<'_>) -> bool {
    let arguments = args(n)
        .into_iter()
        .filter(|a| a.kind() != "comment")
        .collect::<Vec<_>>();
    arguments.len() == 2 && c.symbol(arguments[0]) && c.value(arguments[0]) == "protected"
}

fn protected_hash(c: &Context<'_>, n: Node<'_>) -> bool {
    option_pairs(n).any(|pair| protected_pair(c, pair))
}

fn protected_pair(c: &Context<'_>, n: Node<'_>) -> bool {
    n.kind() == "pair"
        && n.child_by_field_name("key")
            .is_some_and(|key| c.symbol(key) && c.value(key) == "protected")
}
