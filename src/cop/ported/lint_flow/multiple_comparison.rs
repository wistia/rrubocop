use super::super::context::*;
use tree_sitter::Node;
pub(super) fn multiple_comparison(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if !comparison(c, n) {
        return vec![];
    }
    let Some(center) = comparison_center(c, n) else {
        return vec![];
    };
    vec![
        Finding::new(n, "Use the `&&` operator to compare multiple values.").replace_with(
            c,
            center,
            || format!("{} && {}", c.text(center), c.text(center)),
        ),
    ]
}
fn comparison(c: &Context<'_>, n: Node<'_>) -> bool {
    ["<", ">", "<=", ">="].contains(&operator(c, n))
}
fn comparison_center<'a>(c: &Context<'_>, n: Node<'a>) -> Option<Node<'a>> {
    let left = field(n, "left").filter(|l| l.kind() == "binary" && comparison(c, *l))?;
    let center = field(left, "right")?;
    if center.kind() == "binary" && ["&", "|", "^"].contains(&operator(c, center)) {
        None
    } else {
        Some(center)
    }
}
