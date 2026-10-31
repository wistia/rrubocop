use crate::cop::shared::node_bytes;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub(super) fn reserved_assignment(source: &SourceFile, node: Node<'_>) -> bool {
    let name = node_bytes(source, node);
    if !matches!(node.kind(), "identifier" | "hash_key_symbol")
        || name.len() != 2
        || name[0] != b'_'
    {
        return false;
    }
    (b'1'..=b'9').contains(&name[1]) && is_binding(node)
}
fn is_binding(node: Node<'_>) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    match parent.kind() {
        "assignment" | "operator_assignment" => parent.child_by_field_name("left") == Some(node),
        "for" | "match_pattern" | "in_clause" => {
            parent.child_by_field_name("pattern") == Some(node)
        }
        "method" | "singleton_method" => parent.child_by_field_name("name") == Some(node),
        "method_parameters" | "block_parameters" | "lambda_parameters" | "exception_variable" => {
            true
        }
        "keyword_parameter"
        | "optional_parameter"
        | "splat_parameter"
        | "hash_splat_parameter"
        | "block_parameter" => parent.child_by_field_name("name") == Some(node),
        "keyword_pattern" => keyword_binding(parent, node),
        "left_assignment_list"
        | "destructured_left_assignment"
        | "rest_assignment"
        | "destructured_parameter"
        | "array_pattern"
        | "hash_pattern"
        | "find_pattern"
        | "alternative_pattern"
        | "as_pattern" => is_binding(parent),
        _ => false,
    }
}
fn keyword_binding(parent: Node<'_>, node: Node<'_>) -> bool {
    if let Some(value) = parent.child_by_field_name("value") {
        value == node && is_binding(parent)
    } else {
        parent.child_by_field_name("key") == Some(node) && is_binding(parent)
    }
}
