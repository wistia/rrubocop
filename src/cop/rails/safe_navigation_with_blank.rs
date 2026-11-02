//! Remove conditional safe navigation where nil must participate in Rails blankness.
use crate::cop::shared::{argument_nodes, call_method_name, node_bytes, push_replace};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct SafeNavigationWithBlank;
impl Cop for SafeNavigationWithBlank {
    fn name(&self) -> &'static str {
        "Rails/SafeNavigationWithBlank"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn safe_autocorrect(&self) -> bool {
        false
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &[
            "if",
            "unless",
            "if_modifier",
            "unless_modifier",
            "conditional",
        ]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(condition) = node.child_by_field_name("condition") else {
            return;
        };
        if call_method_name(source, condition) != Some(b"blank?")
            || !argument_nodes(condition).is_empty()
            || condition.child_by_field_name("block").is_some()
        {
            return;
        }
        let Some(dot) = condition.child_by_field_name("operator") else {
            return;
        };
        if node_bytes(source, dot) != b"&." {
            return;
        }
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            "Avoid calling `blank?` with the safe navigation operator in conditionals.".into(),
        );
        d.highlight_length =
            super::blank::highlight_length(source, node.start_byte(), node.end_byte());
        d.correctable = !config.autocorrect_disabled();
        if d.correctable {
            d.corrected = push_replace(
                &mut corrections,
                dot.start_byte(),
                dot.end_byte(),
                ".",
                self.name(),
            );
        }
        diagnostics.push(d);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        SafeNavigationWithBlank,
        "cops/rails/safe_navigation_with_blank"
    );
}
