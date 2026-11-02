//! Ruby send ranges exclude an attached block and its separating trivia.
use tree_sitter::Node;

pub(super) fn end(node: Node<'_>) -> usize {
    if node.kind() == "call" {
        node.child_by_field_name("arguments")
            .or_else(|| crate::cop::shared::method_node(node))
            .map_or(node.end_byte(), |node| node.end_byte())
    } else {
        node.end_byte()
    }
}
