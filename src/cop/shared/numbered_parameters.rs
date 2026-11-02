use super::{named_kids, node_bytes};
use crate::parse::source::SourceFile;
use tree_sitter::Node;

fn children(node: Node<'_>) -> Vec<Node<'_>> {
    named_kids(node)
        .into_iter()
        .filter(|n| !matches!(n.kind(), "comment" | "empty_statement"))
        .collect()
}

pub(crate) fn numbered_reference(source: &SourceFile, node: Node<'_>) -> Option<u8> {
    if node.kind() == "identifier" {
        if node.parent().is_some_and(|p| {
            p.child_by_field_name("method") == Some(node) || matches!(p.kind(), "alias" | "undef")
        }) {
            return None;
        }
    } else {
        return None;
    }
    match node_bytes(source, node) {
        [b'_', digit @ b'1'..=b'9'] => Some(*digit - b'0'),
        _ => None,
    }
}

pub(crate) fn outer_scope_values(node: Node<'_>) -> Option<Vec<Node<'_>>> {
    match node.kind() {
        "block" | "do_block" | "lambda" | "method" | "alias" | "undef" => Some(Vec::new()),
        "singleton_method" => Some(node.child_by_field_name("object").into_iter().collect()),
        "singleton_class" => Some(node.child_by_field_name("value").into_iter().collect()),
        "class" | "module" => Some(
            [
                node.child_by_field_name("name"),
                node.child_by_field_name("superclass"),
            ]
            .into_iter()
            .flatten()
            .collect(),
        ),
        _ => None,
    }
}

pub fn numbered_parameter_arity(source: &SourceFile, node: Node<'_>) -> u8 {
    if let Some(headers) = outer_scope_values(node) {
        return headers
            .into_iter()
            .map(|n| numbered_parameter_arity(source, n))
            .max()
            .unwrap_or(0);
    }
    numbered_reference(source, node).unwrap_or(0).max(
        children(node)
            .into_iter()
            .map(|n| numbered_parameter_arity(source, n))
            .max()
            .unwrap_or(0),
    )
}
