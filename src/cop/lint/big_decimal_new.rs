use tree_sitter::Node;

use crate::cop::shared::{call_method_name, call_receiver, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Lint/BigDecimalNew — `BigDecimal.new` is deprecated.
pub struct BigDecimalNew;

fn is_big_decimal_new<'a>(source: &SourceFile, node: Node<'a>) -> Option<(Node<'a>, Node<'a>)> {
    if call_method_name(source, node) != Some(b"new") {
        return None;
    }
    let recv = call_receiver(node)?;
    if node_bytes(source, recv) != b"BigDecimal" {
        return None;
    }
    Some((recv, node.child_by_field_name("method").unwrap_or(node)))
}

impl Cop for BigDecimalNew {
    fn name(&self) -> &'static str {
        "Lint/BigDecimalNew"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }

    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"new"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some((recv, method)) = is_big_decimal_new(source, node) else {
            return;
        };
        let (line, col) = source.offset_to_line_col(method.start_byte());
        let mut diag = self.diagnostic(
            source,
            line,
            col,
            "`BigDecimal.new()` is deprecated. Use `BigDecimal()` instead.".to_string(),
        );
        if let Some(corr) = corrections.as_mut() {
            corr.push(Correction {
                start: recv.end_byte(),
                end: method.end_byte(),
                replacement: String::new(),
                cop_name: self.name(),
                cop_index: 0,
                owner: None,
            });
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}
