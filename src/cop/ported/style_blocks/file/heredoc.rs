use super::super::super::context::*;
use tree_sitter::Node;

pub(super) fn suffix(
    c: &Context<'_>,
    call: Node<'_>,
    target: Node<'_>,
    content: Option<Node<'_>>,
) -> String {
    let references = references(call, content);
    pairs(call)
        .into_iter()
        .filter(|(opener, body)| {
            references.contains(opener) && body.end_byte() <= target.end_byte()
        })
        .map(|(_, body)| c.text(body))
        .collect()
}

fn pairs(call: Node<'_>) -> Vec<(Node<'_>, Node<'_>)> {
    let root = ancestors(call).last().unwrap_or(call);
    let mut openers = nodes_of_kind(root, "heredoc_beginning");
    let mut bodies = nodes_of_kind(root, "heredoc_body");
    openers.sort_by_key(|node| node.start_byte());
    bodies.sort_by_key(|node| node.start_byte());
    openers.into_iter().zip(bodies).collect()
}

fn references<'n>(call: Node<'n>, content: Option<Node<'n>>) -> Vec<Node<'n>> {
    args(call)
        .first()
        .copied()
        .into_iter()
        .chain(content)
        .flat_map(|node| nodes_of_kind(node, "heredoc_beginning"))
        .collect()
}

fn nodes_of_kind<'n>(root: Node<'n>, kind: &str) -> Vec<Node<'n>> {
    descendants(root)
        .filter(|node| node.kind() == kind)
        .collect()
}
