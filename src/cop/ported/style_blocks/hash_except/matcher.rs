use super::super::super::context::*;
use tree_sitter::Node;
pub(super) fn except_key<'a>(c: &Context<'_>, n: Node<'a>, key: &str) -> Option<(Node<'a>, bool)> {
    if n.kind() == "unary" && operator(c, n) == "!" {
        return negated_key(c, n, key);
    }
    let (left, right, method) = comparison_parts(c, n)?;
    if ["==", "!=", "eql?"].contains(&method) {
        return equality_key(c, left, right, method, key);
    }
    inclusion_key(c, left, right, method, key)
}
fn comparison_parts<'a, 'c>(
    c: &'c Context<'_>,
    n: Node<'a>,
) -> Option<(Node<'a>, Node<'a>, &'c str)> {
    if n.kind() == "binary" {
        Some((field(n, "left")?, field(n, "right")?, operator(c, n)))
    } else {
        Some((field(n, "receiver")?, single_argument(c, n)?, c.method(n)))
    }
}
fn single_argument<'n>(c: &Context<'_>, n: Node<'n>) -> Option<Node<'n>> {
    let args = semantic_args(n);
    (args.len() == 1 && block(n).is_none() && !safe(c, n)).then(|| args[0])
}
fn equality_key<'a>(
    c: &Context<'_>,
    left: Node<'a>,
    right: Node<'a>,
    method: &str,
    key: &str,
) -> Option<(Node<'a>, bool)> {
    let other = if c.text(left) == key {
        right
    } else if c.text(right) == key {
        left
    } else {
        return None;
    };
    if method != "eql?" && !c.symbol(other) && !matches!(other.kind(), "string" | "character") {
        return None;
    }
    Some((other, method != "!="))
}
fn inclusion_key<'a>(
    c: &Context<'_>,
    left: Node<'a>,
    right: Node<'a>,
    method: &str,
    key: &str,
) -> Option<(Node<'a>, bool)> {
    let active = c.config.get_bool("ActiveSupportExtensionsEnabled", false);
    match method {
        "include?" if c.text(right) == key => Some((left, true)),
        "exclude?" if c.text(right) == key && active => Some((left, false)),
        "in?" if c.text(left) == key && active => Some((right, true)),
        _ => None,
    }
}

fn negated_key<'n>(c: &Context<'_>, n: Node<'n>, key: &str) -> Option<(Node<'n>, bool)> {
    let operand = field(n, "operand")?;
    let (k, b) = except_key(c, operand, key)?;
    let kind = if ["include?", "in?", "exclude?"].contains(&c.method(operand))
        && c.config.get_bool("ActiveSupportExtensionsEnabled", false)
    {
        true
    } else {
        !b
    };
    Some((k, kind))
}
