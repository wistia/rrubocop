use crate::cop::shared::{call_method_name, call_receiver};
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub(super) fn unsafe_enclosing_use(source: &SourceFile, node: Node<'_>) -> bool {
    if node
        .child_by_field_name("right")
        .and_then(|n| call_method_name(source, n))
        == Some(b"empty?")
    {
        return true;
    }
    let mut parent = node.parent();
    while let Some(n) = parent {
        if unsafe_ancestor(source, n) {
            return true;
        }
        parent = n.parent();
    }
    false
}

fn unsafe_ancestor(source: &SourceFile, n: Node<'_>) -> bool {
    if n.kind() == "unary" {
        return true;
    }
    if n.kind() == "assignment"
        && n.child_by_field_name("left")
            .is_some_and(|l| matches!(l.kind(), "call" | "element_reference"))
    {
        return true;
    }
    if n.kind() == "call"
        && (call_receiver(n).is_none()
            || matches!(
                call_method_name(source, n),
                Some(b"to_i" | b"to_s" | b"to_f" | b"nil?" | b"!")
            ))
    {
        return true;
    }
    false
}
