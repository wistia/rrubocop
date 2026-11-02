//! Express negated assertions with the dedicated Rails assertion method.
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, collect_comments, method_node, node_text,
    push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct AssertNot;
impl Cop for AssertNot {
    fn name(&self) -> &'static str {
        "Rails/AssertNot"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"assert"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if call_method_name(source, node) != Some(b"assert") || call_receiver(node).is_some() {
            return;
        }
        let args = argument_nodes(node);
        let Some(negation) = args.first() else {
            return;
        };
        if negation.kind() != "unary" {
            return;
        }
        let Some(operator) = negation.child_by_field_name("operator") else {
            return;
        };
        if !matches!(node_text(source, operator).as_str(), "!" | "not") {
            return;
        }
        let Some(operand) = negation.child_by_field_name("operand") else {
            return;
        };
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            "Prefer `assert_not` over `assert !`.".into(),
        );
        let end = node
            .child_by_field_name("arguments")
            .map_or(node.end_byte(), |n| n.end_byte());
        d.highlight_length = super::blank::highlight_length(source, node.start_byte(), end);
        let Some(method) = method_node(node) else {
            return;
        };
        let between = &source.as_bytes()[method.end_byte()..operator.start_byte()];
        let supported = operator.end_byte() - operator.start_byte() == 1
            && (between.first() == Some(&b'(') || between.first() == Some(&b' '))
            && !between.contains(&b'\n')
            && !collect_comments(node)
                .iter()
                .any(|c| c.start_byte() < operand.start_byte());
        d.correctable = supported && !config.autocorrect_disabled();
        if d.correctable {
            d.corrected = push_replace(
                &mut corrections,
                method.start_byte(),
                method.end_byte(),
                "assert_not",
                self.name(),
            );
            let mut op_start = operator.start_byte();
            while op_start > method.end_byte() + 1 && source.as_bytes()[op_start - 1] == b' ' {
                op_start -= 1;
            }
            push_replace(
                &mut corrections,
                op_start,
                operand.start_byte(),
                "",
                self.name(),
            );
        }
        diagnostics.push(d);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AssertNot, "cops/rails/assert_not");
}
