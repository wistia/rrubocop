//! Style/VariableInterpolation — prefer #{@var} over #@var.

use tree_sitter::Node;

use crate::cop::shared::node_bytes;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct VariableInterpolation;

impl Cop for VariableInterpolation {
    fn name(&self) -> &'static str {
        "Style/VariableInterpolation"
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
        let text = node_bytes(source, node);
        if !is_bare_var_interp(text) {
            return;
        }
        report(self, source, node, text, diagnostics, &mut corrections);
    }
}

fn is_bare_var_interp(text: &[u8]) -> bool {
    if text.starts_with(b"#{") || text.len() < 2 {
        return false;
    }
    text[0] == b'#' && (text[1] == b'@' || text[1] == b'$')
}

fn report(
    cop: &VariableInterpolation,
    source: &SourceFile,
    node: Node<'_>,
    text: &[u8],
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (line, col) = source.offset_to_line_col(node.start_byte());
    let mut diag = cop.diagnostic(
        source,
        line,
        col,
        "Use braces around interpolated variables.".to_string(),
    );
    if let Some(corr) = corrections.as_mut() {
        let inner = String::from_utf8_lossy(&text[1..]);
        corr.push(Correction {
            start: node.start_byte(),
            end: node.end_byte(),
            replacement: format!("#{{{inner}}}"),
            cop_name: cop.name(),
            cop_index: 0,
            owner: None,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}
