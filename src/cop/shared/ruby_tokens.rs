use super::{for_each_descendant, node_bytes};
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub(crate) fn atomic(source: &SourceFile, node: Node<'_>) -> bool {
    if matches!(node.kind(), "integer" | "float" | "rational" | "complex") {
        return true;
    }
    if node.kind() != "string"
        || !node_bytes(source, node)
            .first()
            .is_some_and(|b| matches!(b, b'\'' | b'"'))
    {
        return false;
    }
    let mut dynamic = false;
    for_each_descendant(node, |n| dynamic |= n.kind() == "interpolation");
    if dynamic {
        return false;
    }
    let mut escaped = false;
    for byte in node_bytes(source, node) {
        if *byte == b'\n' && !escaped {
            return false;
        }
        if *byte == b'\\' {
            escaped = !escaped;
        } else if *byte != b'\r' {
            escaped = false;
        }
    }
    true
}

pub(crate) fn nodes<'a>(source: &SourceFile, root: Node<'a>) -> Vec<Node<'a>> {
    fn walk<'a>(source: &SourceFile, node: Node<'a>, output: &mut Vec<Node<'a>>) {
        if node.is_missing() || node.kind() == "uninterpreted" {
            return;
        }
        if atomic(source, node) || node.child_count() == 0 {
            output.push(node);
        } else {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                walk(source, child, output);
            }
        }
    }
    let mut output = Vec::new();
    walk(source, root, &mut output);
    output
}
