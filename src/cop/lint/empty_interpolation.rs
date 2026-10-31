//! Lint/EmptyInterpolation — `#{}` with empty body.

use tree_sitter::Node;

use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct EmptyInterpolation;

fn is_empty_interp(node: Node<'_>) -> bool {
    let mut cur = node.walk();
    node.named_children(&mut cur).next().is_none()
}

impl Cop for EmptyInterpolation {
    fn name(&self) -> &'static str {
        "Lint/EmptyInterpolation"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["interpolation"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if !is_empty_interp(node) {
            return;
        }
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut diag = self.diagnostic(
            source,
            line,
            col,
            "Empty interpolation detected.".to_string(),
        );
        if let Some(corr) = corrections.as_mut() {
            corr.push(Correction {
                start: node.start_byte(),
                end: node.end_byte(),
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
