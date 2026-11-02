use super::super::context::*;
use tree_sitter::Node;
pub(super) fn identity(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let Some((left, right)) = comparison(c, n) else {
        return vec![];
    };
    if !object_id(c, left) || !object_id(c, right) {
        return vec![];
    }
    if field(left, "receiver").is_none() || field(right, "receiver").is_none() {
        return vec![];
    }
    let finding = Finding::send(n, "Use `equal?` instead `==` when comparing `object_id`.");
    vec![correct_identity(c, finding, left, right)]
}
fn object_id(c: &Context<'_>, n: Node<'_>) -> bool {
    c.method(n) == "object_id" && block(n).is_none()
}
fn comparison<'a>(c: &Context<'_>, n: Node<'a>) -> Option<(Node<'a>, Node<'a>)> {
    if n.kind() == "binary" && operator(c, n) == "==" {
        field(n, "left").zip(field(n, "right"))
    } else if c.method(n) == "==" {
        field(n, "receiver").zip(semantic_args(n).first().copied())
    } else {
        None
    }
}
fn correct_identity(c: &Context<'_>, finding: Finding, left: Node<'_>, right: Node<'_>) -> Finding {
    match field(left, "receiver").zip(field(right, "receiver")) {
        Some((l, r)) => {
            let (start, end) = (finding.start, finding.end);
            finding.edit_with(c, start, end, || {
                format!("{}.equal?({})", c.text(l), c.text(r))
            })
        }
        None => finding,
    }
}
