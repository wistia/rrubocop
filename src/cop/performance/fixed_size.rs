//! Avoid computing a literal object's statically known size.
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, for_each_descendant, named_kids, node_text,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct FixedSize;
impl Cop for FixedSize {
    fn name(&self) -> &'static str {
        "Performance/FixedSize"
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"count", b"length", b"size"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if !matches!(
            call_method_name(source, node),
            Some(b"count" | b"length" | b"size")
        ) || allowed_parent(node)
            || node.child_by_field_name("block").is_some()
        {
            return;
        }
        let Some(receiver) = call_receiver(node) else {
            return;
        };
        if !matches!(
            receiver.kind(),
            "array"
                | "string_array"
                | "symbol_array"
                | "hash"
                | "string"
                | "chained_string"
                | "simple_symbol"
                | "delimited_symbol"
        ) {
            return;
        }
        if matches!(
            receiver.kind(),
            "string" | "chained_string" | "delimited_symbol"
        ) && interpolated(receiver)
        {
            return;
        }
        if named_kids(receiver).iter().any(|n| {
            matches!(
                n.kind(),
                "splat_argument" | "hash_splat_argument" | "hash_splat"
            )
        }) {
            return;
        }
        if argument_nodes(node)
            .first()
            .is_some_and(|n| n.kind() != "string" || interpolated(*n))
        {
            return;
        }
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            "Do not compute the size of statically sized objects.".into(),
        );
        let text = node_text(source, node);
        d.highlight_length = text.lines().next().unwrap_or("").chars().count();
        diagnostics.push(d);
    }
}
fn interpolated(node: Node<'_>) -> bool {
    let mut yes = false;
    for_each_descendant(node, |n| yes |= n.kind() == "interpolation");
    yes
}
fn allowed_parent(node: Node<'_>) -> bool {
    let mut parent = node.parent();
    while let Some(node) = parent {
        if matches!(node.kind(), "block" | "do_block") {
            return true;
        }
        if node.kind() == "assignment"
            && node
                .child_by_field_name("left")
                .is_some_and(|left| matches!(left.kind(), "constant" | "scope_resolution"))
        {
            return true;
        }
        parent = node.parent();
    }
    false
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FixedSize, "cops/performance/fixed_size");
}
