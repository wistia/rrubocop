use super::pattern::{children, numbered, outer_values};
use crate::cop::shared::{call_method_name, node_bytes};
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub(super) fn shadow_parameter(node: Node<'_>) -> bool {
    let Some(parent) = node.parent().filter(|p| p.kind() == "block_parameters") else {
        return false;
    };
    parent
        .children_by_field_name("locals", &mut parent.walk())
        .any(|n| n == node)
}
fn pattern_binds(source: &SourceFile, node: Node<'_>, name: &[u8]) -> bool {
    if matches!(
        node.kind(),
        "variable_reference_pattern" | "expression_reference_pattern"
    ) {
        return false;
    }
    if node.kind() == "identifier" {
        return node_bytes(source, node) == name;
    }
    let parts = children(node);
    if node.kind() == "keyword_pattern" && parts.len() == 1 && parts[0].kind() == "hash_key_symbol"
    {
        return node_bytes(source, parts[0]) == name;
    }
    parts
        .into_iter()
        .filter(|n| n.kind() != "hash_key_symbol")
        .any(|n| pattern_binds(source, n, name))
}
pub(super) fn pattern_assignment(source: &SourceFile, node: Node<'_>, name: &[u8]) -> bool {
    matches!(
        node.kind(),
        "match_pattern" | "match_pattern_guard" | "in_clause"
    ) && node
        .child_by_field_name("pattern")
        .is_some_and(|p| pattern_binds(source, p, name))
        || children(node)
            .into_iter()
            .any(|n| pattern_assignment(source, n, name))
}
pub(super) fn nested_numbered(source: &SourceFile, node: Node<'_>, nested: bool) -> bool {
    let nested = nested || matches!(node.kind(), "block" | "do_block" | "lambda");
    nested
        && numbered(source, node).is_some()
        && crate::cop::ported::ruby_local_identifier(source, node)
        || children(node)
            .into_iter()
            .any(|n| nested_numbered(source, n, nested))
}
pub(super) fn shorthand_numbered(source: &SourceFile, node: Node<'_>) -> bool {
    node.kind() == "hash_key_symbol"
        && matches!(node_bytes(source, node), b"_1" | b"_2")
        && node
            .parent()
            .is_some_and(|p| p.kind() == "pair" && p.child_by_field_name("value").is_none())
        || children(node)
            .into_iter()
            .any(|n| shorthand_numbered(source, n))
}
pub(super) fn reflective(source: &SourceFile, node: Node<'_>) -> bool {
    let method = if node.kind() == "call" {
        call_method_name(source, node)
    } else if node.kind() == "identifier"
        && !crate::cop::ported::ruby_local_identifier(source, node)
    {
        Some(node_bytes(source, node))
    } else {
        None
    };
    method.is_some_and(|m| matches!(m, b"binding" | b"local_variables" | b"eval"))
        || children(node).into_iter().any(|n| reflective(source, n))
}
pub(super) fn different_next(source: &SourceFile, node: Node<'_>, name: &[u8]) -> bool {
    if matches!(
        node.kind(),
        "while" | "until" | "for" | "while_modifier" | "until_modifier"
    ) {
        return false;
    }
    if let Some(headers) = outer_values(node) {
        return headers.into_iter().any(|n| different_next(source, n, name));
    }
    if node.kind() == "next" {
        let values = children(node)
            .into_iter()
            .find(|n| n.kind() == "argument_list")
            .map(children)
            .unwrap_or_default();
        return values.len() != 1
            || values[0].kind() != "identifier"
            || node_bytes(source, values[0]) != name;
    }
    children(node)
        .into_iter()
        .any(|n| different_next(source, n, name))
}
