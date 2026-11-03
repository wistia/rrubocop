//! Ruby send ranges exclude an attached block and its separating trivia.
use tree_sitter::Node;

pub fn end(node: Node<'_>) -> usize {
    if node.kind() == "call" {
        node.child_by_field_name("arguments")
            .or_else(|| crate::cop::shared::method_node(node))
            .map_or(node.end_byte(), |node| node.end_byte())
    } else {
        node.end_byte()
    }
}

pub fn is_setter_call(node: Node<'_>) -> bool {
    if !matches!(node.kind(), "call" | "element_reference") {
        return false;
    }
    let mut current = node;
    while let Some(parent) = current.parent() {
        if matches!(
            parent.kind(),
            "left_assignment_list" | "destructured_left_assignment"
        ) {
            current = parent;
            continue;
        }
        return parent.kind() == "assignment"
            && parent.child_by_field_name("left") == Some(current)
            || parent.kind() == "for" && parent.child_by_field_name("pattern") == Some(current);
    }
    false
}
