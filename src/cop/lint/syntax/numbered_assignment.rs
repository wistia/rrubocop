use tree_sitter::Node;

use crate::cop::shared::node_bytes;
use crate::parse::source::SourceFile;

pub(super) fn reserved_assignment(source: &SourceFile, node: Node<'_>) -> bool {
    let name = node_bytes(source, node);
    if node.kind() != "identifier" || name.len() != 2 || name[0] != b'_' {
        return false;
    }
    if !(b'1'..=b'9').contains(&name[1]) {
        return false;
    }
    is_target(node)
}

fn is_target(node: Node<'_>) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    match parent.kind() {
        "assignment" | "operator_assignment" => parent.child_by_field_name("left") == Some(node),
        "left_assignment_list" | "destructured_left_assignment" | "rest_assignment" => {
            is_target(parent)
        }
        _ => false,
    }
}
