//! Avoid expanding a string star when constructing Arel column lists.
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, named_kids, node_text, push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct ArelStar;
impl Cop for ArelStar {
    fn name(&self) -> &'static str {
        "Rails/ArelStar"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn safe_autocorrect(&self) -> bool {
        false
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["element_reference", "call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"[]"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let (receiver, args) = if node.kind() == "element_reference" {
            let Some(receiver) = node.child_by_field_name("object") else {
                return;
            };
            (
                receiver,
                named_kids(node)
                    .into_iter()
                    .filter(|n| *n != receiver && n.kind() != "comment")
                    .collect::<Vec<_>>(),
            )
        } else {
            if call_method_name(source, node) != Some(b"[]") {
                return;
            }
            let Some(receiver) = call_receiver(node) else {
                return;
            };
            (receiver, argument_nodes(node))
        };
        if args.len() != 1
            || !matches!(args[0].kind(), "string" | "chained_string")
            || crate::cop::ported::static_literal(source, args[0]).as_deref() != Some("*")
        {
            return;
        }
        let constant = matches!(receiver.kind(), "constant" | "scope_resolution");
        let arel = (call_method_name(source, receiver) == Some(b"arel_table")
            || receiver.kind() == "identifier"
                && node_text(source, receiver) == "arel_table"
                && !crate::cop::ported::local_identifier(source, receiver))
            && argument_nodes(receiver).is_empty()
            && receiver.child_by_field_name("block").is_none();
        if !constant && !arel {
            return;
        }
        if node
            .child_by_field_name("operator")
            .is_some_and(|op| node_text(source, op) == "&.")
        {
            return;
        }
        let star = args[0];
        let (line, col) = source.offset_to_line_col(star.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            "Use `Arel.star` instead of `\"*\"` for expanded column lists.".into(),
        );
        d.highlight_length = node_text(source, star).chars().count();
        d.correctable = !config.autocorrect_disabled();
        d.corrected = push_replace(
            &mut corrections,
            star.start_byte(),
            star.end_byte(),
            "Arel.star",
            self.name(),
        );
        diagnostics.push(d);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ArelStar, "cops/rails/arel_star");
}
