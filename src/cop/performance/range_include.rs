//! Use range bounds rather than enumerating the range.
use crate::cop::shared::{call_method_name, call_receiver, method_node, node_text, push_replace};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct RangeInclude;
impl Cop for RangeInclude {
    fn name(&self) -> &'static str {
        "Performance/RangeInclude"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn safe_autocorrect(&self) -> bool {
        false
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"include?", b"member?"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if !matches!(
            call_method_name(source, node),
            Some(b"include?" | b"member?")
        ) {
            return;
        }
        let Some(receiver) = call_receiver(node) else {
            return;
        };
        let receiver =
            if receiver.kind() == "parenthesized_statements" && receiver.named_child_count() == 1 {
                receiver.named_child(0).unwrap()
            } else {
                receiver
            };
        if receiver.kind() != "range" {
            return;
        }
        let Some(method) = method_node(node) else {
            return;
        };
        let (line, col) = source.offset_to_line_col(method.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            format!(
                "Use `Range#cover?` instead of `Range#{}`.",
                node_text(source, method)
            ),
        );
        d.highlight_length = method.end_byte() - method.start_byte();
        d.corrected = push_replace(
            &mut corrections,
            method.start_byte(),
            method.end_byte(),
            "cover?",
            self.name(),
        );
        d.correctable &= !config.autocorrect_disabled();
        diagnostics.push(d);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(RangeInclude, "cops/performance/range_include");
}
