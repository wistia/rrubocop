use super::super::context::*;
use tree_sitter::Node;
pub(super) fn identity(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let Some((left, right)) = comparison(c, n) else {
        return vec![];
    };
    if c.method(left) != "object_id" || c.method(right) != "object_id" {
        return vec![];
    }
    if field(left, "receiver").is_none() || field(right, "receiver").is_none() {
        return vec![];
    }
    let finding = Finding::new(n, "Use `equal?` instead `==` when comparing `object_id`.");
    vec![correct_identity(c, finding, n, left, right)]
}
fn comparison<'a>(c: &Context<'_>, n: Node<'a>) -> Option<(Node<'a>, Node<'a>)> {
    if n.kind() == "binary" && operator(c, n) == "==" {
        field(n, "left").zip(field(n, "right"))
    } else if c.method(n) == "==" {
        field(n, "receiver").zip(args(n).first().copied())
    } else {
        None
    }
}
fn correct_identity(
    c: &Context<'_>,
    finding: Finding,
    n: Node<'_>,
    left: Node<'_>,
    right: Node<'_>,
) -> Finding {
    match field(left, "receiver").zip(field(right, "receiver")) {
        Some((l, r)) => finding.replace(n, format!("{}.equal?({})", c.text(l), c.text(r))),
        None => finding,
    }
}
