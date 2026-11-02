use super::*;

pub(super) fn count(c: &Context<'_>, n: Node<'_>) -> u8 {
    lexical_children(n)
        .into_iter()
        .map(|child| count(c, child))
        .max()
        .unwrap_or(0)
        .max(parameter(c, n))
}

fn parameter(c: &Context<'_>, n: Node<'_>) -> u8 {
    if n.kind() != "identifier"
        || n.parent()
            .is_some_and(|p| matches!(p.kind(), "alias" | "undef") || field(p, "method") == Some(n))
    {
        return 0;
    }
    match c.text(n).as_bytes() {
        [b'_', number @ b'1'..=b'9'] => number - b'0',
        _ => 0,
    }
}

fn lexical_children(n: Node<'_>) -> Vec<Node<'_>> {
    match n.kind() {
        "class" | "module" => ["name", "superclass"]
            .into_iter()
            .filter_map(|name| field(n, name))
            .collect(),
        "singleton_method" => field(n, "object").into_iter().collect(),
        "singleton_class" => field(n, "value").into_iter().collect(),
        "block" | "do_block" | "lambda" | "method" => vec![],
        _ => kids(n),
    }
}
