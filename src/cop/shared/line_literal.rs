use super::node_bytes;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub(crate) fn is_line_literal(source: &SourceFile, node: Node<'_>) -> bool {
    node.kind() == "identifier"
        && node_bytes(source, node) == b"__LINE__"
        && !node.parent().is_some_and(|parent| {
            parent.child_by_field_name("method") == Some(node)
                || matches!(parent.kind(), "alias" | "undef")
                || matches!(parent.kind(), "method" | "singleton_method")
                    && parent.child_by_field_name("name") == Some(node)
        })
}
