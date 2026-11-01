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
    arguments.first().is_some_and(|argument| {
        c.static_symbol(*argument).as_deref() == Some("protected")
            && (arguments.len() == 2 || implicit_hash(&arguments[1..]))
    })
}

fn protected_hash(c: &Context<'_>, n: Node<'_>) -> bool {
    let arguments: Vec<_> = args(n)
        .into_iter()
        .filter(|argument| argument.kind() != "comment")
        .collect();
    (implicit_hash(&arguments) || arguments.len() == 1 && arguments[0].kind() == "hash")
        && option_pairs(n).any(|pair| protected_pair(c, pair))
}

fn protected_pair(c: &Context<'_>, n: Node<'_>) -> bool {
    n.kind() == "pair"
        && n.child_by_field_name("key")
            .is_some_and(|key| c.static_symbol(key).as_deref() == Some("protected"))
}

fn implicit_hash(arguments: &[Node<'_>]) -> bool {
    !arguments.is_empty()
        && arguments
            .iter()
            .all(|argument| matches!(argument.kind(), "pair" | "hash_splat_argument"))
}
