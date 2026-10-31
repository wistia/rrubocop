use super::super::context::*;
use tree_sitter::Node;
pub(super) fn empty_when(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if field(n, "body").is_some_and(|b| !kids(b).is_empty()) {
        return vec![];
    }
    if c.config.get_bool("AllowComments", true) && when_comments(n) {
        return vec![];
    }
    vec![Finding::new(n, "Avoid `when` branches without a body.")]
}

fn when_comments(n: Node<'_>) -> bool {
    if descendants(n).any(|d| d.kind() == "comment") {
        return true;
    }
    descendants(ancestors(n).last().unwrap_or(n))
        .any(|d| d.kind() == "comment" && d.start_position().row == n.end_position().row)
}
