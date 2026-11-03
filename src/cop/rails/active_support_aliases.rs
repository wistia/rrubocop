use crate::cop::shared::ruby_expression::plain_string;
use crate::cop::shared::{
    call_method_name, call_receiver, is_setter_call, method_node, push_replace, send_end,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct ActiveSupportAliases;
impl Cop for ActiveSupportAliases {
    fn name(&self) -> &'static str {
        "Rails/ActiveSupportAliases"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"starts_with?", b"ends_with?", b"append", b"prepend"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if is_setter_call(node) || crate::cop::shared::ruby_arguments::arguments(node).len() != 1 {
            return;
        }
        let Some(receiver) = call_receiver(node) else {
            return;
        };
        let Some(method) = call_method_name(source, node) else {
            return;
        };
        let preferred = match method {
            b"starts_with?" if plain_string(source, receiver).is_some() => "start_with?",
            b"ends_with?" if plain_string(source, receiver).is_some() => "end_with?",
            b"append" if matches!(receiver.kind(), "array" | "string_array" | "symbol_array") => {
                "<<"
            }
            b"prepend" if matches!(receiver.kind(), "array" | "string_array" | "symbol_array") => {
                "unshift"
            }
            _ => return,
        };
        let Some(selector) = method_node(node) else {
            return;
        };
        let (line, col) = source.offset_to_line_col(selector.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            format!(
                "Use `{preferred}` instead of `{}`.",
                String::from_utf8_lossy(method)
            ),
        );
        d.highlight_length =
            super::blank::highlight_length(source, selector.start_byte(), send_end(node));
        if let Some(s) = config.severity {
            d.severity = s;
        }
        d.correctable = method != b"append" && !config.autocorrect_disabled();
        if d.correctable {
            d.corrected = push_replace(
                &mut corrections,
                selector.start_byte(),
                selector.end_byte(),
                preferred,
                self.name(),
            );
        }
        diagnostics.push(d);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ActiveSupportAliases, "cops/rails/active_support_aliases");
}
