use super::super::super::context::*;
use tree_sitter::Node;

pub(in crate::cop::ported::retained) fn setex(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if c.method(n) != "setex" {
        return vec![];
    }
    let f = Finding::send(
        n,
        "`setex(key, expires_in, value)` is deprecated, please use `set(key, value, ex: expires_in)` instead",
    );
    let Some((expiry, value)) = swappable(c, n) else {
        return vec![f];
    };
    vec![
        f.replace_with(c, method_node(n), || "set")
            .also_edit_with(c, expiry.start_byte(), expiry.end_byte(), || c.text(value))
            .also_edit_with(c, value.start_byte(), value.end_byte(), || {
                format!("ex: {}", c.text(expiry))
            }),
    ]
}

fn swappable<'n>(c: &Context<'_>, n: Node<'n>) -> Option<(Node<'n>, Node<'n>)> {
    let mut a = super::super::arguments(n);
    if a.last().is_some_and(|a| a.kind() == "block_argument") {
        a.pop();
    }
    if a.len() != 3
        || a.iter().any(|a| {
            matches!(
                a.kind(),
                "splat_argument" | "hash_splat_argument" | "forward_argument"
            )
        })
    {
        return None;
    }
    (stable(c, a[1]) && stable(c, a[2])).then_some((a[1], a[2]))
}

fn stable(c: &Context<'_>, n: Node<'_>) -> bool {
    match n.kind() {
        "integer" | "float" | "true" | "false" | "nil" | "simple_symbol" => true,
        "identifier" => local_identifier(c, n),
        "string" | "delimited_symbol" => !descendants(n).any(|n| n.kind() == "interpolation"),
        "unary" => kids(n)
            .iter()
            .all(|n| matches!(n.kind(), "integer" | "float")),
        _ => false,
    }
}

#[cfg(test)]
mod tests;
