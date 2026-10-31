use tree_sitter::Node;

use crate::cop::shared::node_bytes;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Lint/UnifiedInteger — Fixnum/Bignum → Integer.
pub struct UnifiedInteger;

impl Cop for UnifiedInteger {
    fn name(&self) -> &'static str {
        "Lint/UnifiedInteger"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["constant"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let klass = match node_bytes(source, node) {
            b"Fixnum" => "Fixnum",
            b"Bignum" => "Bignum",
            _ => return,
        };
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut diag = self.diagnostic(
            source,
            line,
            col,
            format!("Use `Integer` instead of `{klass}`."),
        );
        if let Some(corr) = corrections.as_deref_mut() {
            corr.push(crate::correction::Correction {
                start: node.start_byte(),
                end: node.end_byte(),
                replacement: "Integer".into(),
                cop_name: self.name(),
                cop_index: 0,
                owner: None,
            });
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }
}
