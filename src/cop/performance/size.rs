//! Count known arrays and hashes with their constant-time size method.
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, method_node, node_text, push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct Size;
impl Cop for Size {
    fn name(&self) -> &'static str {
        "Performance/Size"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"count"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if call_method_name(source, node) != Some(b"count")
            || !argument_nodes(node).is_empty()
            || node.child_by_field_name("block").is_some()
        {
            return;
        }
        let Some(receiver) = call_receiver(node) else {
            return;
        };
        if !known_collection(source, receiver) {
            return;
        }
        let Some(method) = method_node(node) else {
            return;
        };
        let (line, col) = source.offset_to_line_col(method.start_byte());
        let mut d = self.diagnostic(source, line, col, "Use `size` instead of `count`.".into());
        d.highlight_length = method.end_byte() - method.start_byte();
        d.corrected = push_replace(
            &mut corrections,
            method.start_byte(),
            method.end_byte(),
            "size",
            self.name(),
        );
        d.correctable &= !config.autocorrect_disabled();
        diagnostics.push(d);
    }
}
fn known_collection(source: &SourceFile, node: Node<'_>) -> bool {
    if matches!(
        node.kind(),
        "array" | "string_array" | "symbol_array" | "hash"
    ) {
        return true;
    }
    if matches!(call_method_name(source, node), Some(b"to_a" | b"to_h"))
        && argument_nodes(node).is_empty()
        && node.child_by_field_name("block").is_none()
    {
        return true;
    }
    if node.kind() == "element_reference" && node.named_child_count() == 2 {
        return node
            .child_by_field_name("object")
            .is_some_and(|n| matches!(node_text(source, n).as_str(), "Array" | "Hash"));
    }
    matches!(call_method_name(source, node), Some(b"Array" | b"Hash"))
        && call_receiver(node).is_none()
        && argument_nodes(node).len() == 1
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Size, "cops/performance/size");
}
