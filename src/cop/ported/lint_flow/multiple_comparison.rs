use super::super::context::*;
use tree_sitter::Node;
pub(super) fn multiple_comparison(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if !comparison(c, n) {
        return vec![];
    }
    let Some(center) = comparison_center(c, n) else {
        return vec![];
    };
    let finding = Finding::new(n, "Use the `&&` operator to compare multiple values.");
    if field(n, "left").is_some_and(is_call) {
        return vec![finding];
    }
    vec![finding.replace_with(c, center, || {
        format!("{} && {}", c.text(center), c.text(center))
    })]
}
fn comparison(c: &Context<'_>, n: Node<'_>) -> bool {
    if safe(c, n) {
        return false;
    }
    ["<", ">", "<=", ">="].contains(&if is_call(n) {
        c.method(n)
    } else {
        operator(c, n)
    })
}
fn comparison_center<'a>(c: &Context<'_>, n: Node<'a>) -> Option<Node<'a>> {
    let left = field(n, "left").filter(|l| comparison(c, *l))?;
    let center = center_argument(left)?;
    if bitwise(c, center) {
        None
    } else {
        Some(center)
    }
}
fn bitwise(c: &Context<'_>, n: Node<'_>) -> bool {
    let method = if n.kind() == "binary" {
        operator(c, n)
    } else {
        c.method(n)
    };
    ["&", "|", "^"].contains(&method)
}

fn center_argument(left: Node<'_>) -> Option<Node<'_>> {
    if !is_call(left) {
        return field(left, "right");
    }
    let args = semantic_args(left);
    (args.len() == 1 && block(left).is_none()).then(|| args[0])
}
