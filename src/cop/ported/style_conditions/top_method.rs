use super::super::context::*;
use tree_sitter::Node;
pub(super) fn top_method(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if is_call(n) && c.method(n) != "define_method" {
        return vec![];
    }
    if n.parent().is_some_and(|p| p.kind() == "program") {
        vec![Finding::new(n, "Do not define methods at the top-level.")]
    } else {
        vec![]
    }
}
