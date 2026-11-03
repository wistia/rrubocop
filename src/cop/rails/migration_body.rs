use crate::cop::shared::named_kids;
use crate::cop::shared::ruby_expression::{
    regular_send_arguments, regular_send_name, regular_send_receiver,
};
use crate::parse::source::SourceFile;
use tree_sitter::Node;

fn children(node: Node<'_>) -> Vec<Node<'_>> {
    named_kids(node)
        .into_iter()
        .filter(|n| !matches!(n.kind(), "comment" | "heredoc_body" | "empty_statement"))
        .collect()
}
fn container(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "block_body" | "body_statement" | "then" | "else" | "do" | "superclass"
    )
}
fn statements(node: Node<'_>) -> Vec<Node<'_>> {
    if !container(node) {
        return vec![node];
    }
    let nodes = children(node);
    if nodes.len() == 1 && container(nodes[0]) {
        statements(nodes[0])
    } else {
        nodes
    }
}
fn send(source: &SourceFile, node: Node<'_>) -> bool {
    regular_send_name(source, node).is_some() && node.child_by_field_name("block").is_none()
}
fn logical<'n>(source: &SourceFile, node: Node<'n>) -> (Node<'n>, bool) {
    let nodes = statements(node);
    if nodes.len() == 1 {
        (nodes[0], send(source, nodes[0]))
    } else {
        (node, false)
    }
}
fn ast_children<'n>(source: &SourceFile, node: Node<'n>) -> Vec<(Node<'n>, bool)> {
    if let Some(block) = node.child_by_field_name("block") {
        let mut result = vec![(node, true)];
        result.extend(block.child_by_field_name("parameters").map(|n| (n, false)));
        result.extend(
            block
                .child_by_field_name("body")
                .map(|n| logical(source, n)),
        );
        return result;
    }
    if send(source, node) {
        return regular_send_receiver(source, node)
            .flatten()
            .into_iter()
            .chain(regular_send_arguments(source, node).unwrap_or_default())
            .map(|n| (n, send(source, n)))
            .collect();
    }
    children(node)
        .into_iter()
        .map(|n| logical(source, n))
        .collect()
}

pub(super) fn body_children<'n>(source: &SourceFile, body: Node<'n>) -> Vec<(Node<'n>, bool)> {
    let nodes = statements(body);
    if nodes.len() == 1 {
        ast_children(source, nodes[0])
    } else {
        nodes.into_iter().map(|n| (n, send(source, n))).collect()
    }
}
pub(super) fn body_sends<'n>(source: &SourceFile, body: Node<'n>) -> Vec<Node<'n>> {
    let nodes = statements(body);
    if nodes.len() == 1 && send(source, nodes[0]) {
        return nodes;
    }
    body_children(source, body)
        .into_iter()
        .filter_map(|(n, send)| send.then_some(n))
        .collect()
}
