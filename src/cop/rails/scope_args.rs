//! Delay relation construction when declaring an Active Record scope.
use crate::cop::shared::{argument_nodes, call_method_name, call_receiver, push_replace};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct ScopeArgs;
impl Cop for ScopeArgs {
    fn name(&self) -> &'static str {
        "Rails/ScopeArgs"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"scope"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if call_method_name(source, node) != Some(b"scope") || call_receiver(node).is_some() {
            return;
        }
        let args = argument_nodes(node);
        if args.len() != 2 {
            return;
        }
        let value = args[1];
        let send = value.kind() == "call"
            && value.child_by_field_name("block").is_none()
            && value
                .child_by_field_name("operator")
                .is_none_or(|operator| crate::cop::shared::node_bytes(source, operator) != b"&.")
            || value.kind() == "identifier" && !crate::cop::ported::local_identifier(source, value);
        if !send {
            return;
        }
        let (line, col) = source.offset_to_line_col(value.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            "Use `lambda`/`proc` instead of a plain method call.".into(),
        );
        d.highlight_length =
            super::blank::highlight_length(source, value.start_byte(), value.end_byte());
        d.correctable = !config.autocorrect_disabled();
        if d.correctable {
            d.corrected = push_replace(
                &mut corrections,
                value.start_byte(),
                value.start_byte(),
                "-> { ",
                self.name(),
            );
            push_replace(
                &mut corrections,
                value.end_byte(),
                value.end_byte(),
                " }",
                self.name(),
            );
        }
        diagnostics.push(d);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ScopeArgs, "cops/rails/scope_args");
}
