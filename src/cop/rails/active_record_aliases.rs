use crate::cop::shared::{
    argument_nodes, call_method_name, is_setter_call, method_node, push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct ActiveRecordAliases;
impl Cop for ActiveRecordAliases {
    fn name(&self) -> &'static str {
        "Rails/ActiveRecordAliases"
    }
    fn default_severity(&self) -> Severity {
        Severity::Warning
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
        &[b"update_attributes", b"update_attributes!"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if is_setter_call(node) || argument_nodes(node).is_empty() {
            return;
        }
        let Some(method) = call_method_name(source, node) else {
            return;
        };
        let preferred = match method {
            b"update_attributes" => "update",
            b"update_attributes!" => "update!",
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
            super::blank::highlight_length(source, selector.start_byte(), selector.end_byte());
        if let Some(s) = config.severity {
            d.severity = s;
        }
        d.correctable = !config.autocorrect_disabled();
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
