use super::super::context::*;
use tree_sitter::Node;
pub(super) fn empty_when(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if has_body(n) {
        return vec![];
    }
    if c.config.get_bool("AllowComments", true) && when_comments(n) {
        return vec![];
    }
    let mut cursor = n.walk();
    let end = n
        .children_by_field_name("pattern", &mut cursor)
        .last()
        .map_or(n.end_byte(), |pattern| pattern.end_byte());
    vec![Finding::range(
        n.start_byte(),
        end,
        "Avoid `when` branches without a body.",
    )]
}

fn has_body(n: Node<'_>) -> bool {
    field(n, "body").is_some_and(|body| {
        kids(body)
            .iter()
            .any(|node| node.kind() != "empty_statement")
    })
}

fn when_comments(n: Node<'_>) -> bool {
    if descendants(n).any(|d| d.kind() == "comment") {
        return true;
    }
    descendants(ancestors(n).last().unwrap_or(n))
        .any(|d| d.kind() == "comment" && d.start_position().row == n.end_position().row)
}
