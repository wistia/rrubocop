use super::super::super::context::*;
use tree_sitter::Node;

pub(super) fn suffix(
    c: &Context<'_>,
    call: Node<'_>,
    target: Node<'_>,
    content: Option<Node<'_>>,
) -> String {
    let mut references = references(call, content)
        .into_iter()
        .map(|node| node.start_byte())
        .collect::<Vec<_>>();
    references.sort_unstable();
    references.dedup();
    let root = ancestors(call).last().unwrap_or(call);
    let index = crate::cop::shared::heredoc_index::heredoc_index(c.source, root);
    references
        .into_iter()
        .filter_map(|start| index.body_for_opener(start))
        .filter(|body| body.node.end <= target.end_byte())
        .map(|body| {
            std::str::from_utf8(&c.source.as_bytes()[body.node.start..body.node.end]).unwrap_or("")
        })
        .collect()
}

fn references<'n>(call: Node<'n>, content: Option<Node<'n>>) -> Vec<Node<'n>> {
    semantic_args(call)
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
