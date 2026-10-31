use super::super::context::*;
use tree_sitter::Node;
pub(super) fn empty(_c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if kids(n).is_empty() {
        vec![Finding::new(n, "Avoid empty expressions.")]
    } else {
        vec![]
    }
}
