use crate::cop::shared::{named_kids, node_bytes};
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub(super) fn children(node: Node<'_>) -> Vec<Node<'_>> {
    named_kids(node)
        .into_iter()
        .filter(|n| !matches!(n.kind(), "comment" | "empty_statement"))
        .collect()
}
pub(super) fn returned(body: Node<'_>) -> Option<Node<'_>> {
    let statements = if matches!(body.kind(), "block_body" | "body_statement") {
        children(body)
    } else {
        vec![body]
    };
    let mut last = *statements.last()?;
    if statements.len() == 1 && last.kind() == "parenthesized_statements" {
        last = *children(last).last()?;
    }
    (last.kind() == "identifier").then_some(last)
}
pub(super) fn basic_literal(source: &SourceFile, node: Node<'_>) -> bool {
    match node.kind() {
        "integer" | "float" | "rational" | "complex" | "true" | "false" | "nil" | "character"
        | "simple_symbol" => true,
        "identifier" => matches!(node_bytes(source, node), b"__FILE__" | b"__LINE__"),
        "string" | "delimited_symbol" | "heredoc_beginning" => {
            crate::cop::ported::ruby_string_parts(source, node).is_some_and(|p| p.len() <= 1)
        }
        "unary" => {
            node.child_by_field_name("operator")
                .is_some_and(|op| matches!(node_bytes(source, op), b"+" | b"-"))
                && node.child_by_field_name("operand").is_some_and(|n| {
                    matches!(n.kind(), "integer" | "float" | "rational" | "complex")
                })
        }
        _ => false,
    }
}
pub(super) fn parameter_name(node: Node<'_>) -> Option<Node<'_>> {
    if node.kind() == "identifier" {
        return Some(node);
    }
    if matches!(
        node.kind(),
        "optional_parameter"
            | "keyword_parameter"
            | "splat_parameter"
            | "hash_splat_parameter"
            | "block_parameter"
    ) {
        return node.child_by_field_name("name").or_else(|| {
            children(node)
                .into_iter()
                .find(|n| n.kind() == "identifier")
        });
    }
    None
}
fn target_name(source: &SourceFile, node: Node<'_>, name: &[u8]) -> bool {
    match node.kind() {
        "identifier" => node_bytes(source, node) == name,
        "left_assignment_list"
        | "destructured_left_assignment"
        | "destructured_left_assignment_list"
        | "rest_assignment"
        | "exception_variable" => children(node)
            .into_iter()
            .any(|n| target_name(source, n, name)),
        _ => false,
    }
}
pub(super) fn assigned(source: &SourceFile, node: Node<'_>, name: &[u8]) -> bool {
    let target = match node.kind() {
        "assignment" | "operator_assignment" => node.child_by_field_name("left"),
        "for" => node.child_by_field_name("pattern"),
        "rescue" => node.child_by_field_name("variable"),
        _ => None,
    };
    target.is_some_and(|n| target_name(source, n, name))
        || children(node)
            .into_iter()
            .any(|n| assigned(source, n, name))
}
pub(super) fn numbered(source: &SourceFile, node: Node<'_>) -> Option<u8> {
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
pub(super) fn outer_values(node: Node<'_>) -> Option<Vec<Node<'_>>> {
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
pub(super) fn arity(source: &SourceFile, node: Node<'_>) -> u8 {
    if let Some(headers) = outer_values(node) {
        return headers
            .into_iter()
            .map(|n| arity(source, n))
            .max()
            .unwrap_or(0);
    }
    numbered(source, node).unwrap_or(0).max(
        children(node)
            .into_iter()
            .map(|n| arity(source, n))
            .max()
            .unwrap_or(0),
    )
}
