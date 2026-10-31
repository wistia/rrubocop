//! Style/Not — prefer `!` over `not`.

use tree_sitter::Node;

use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct Not;

impl Cop for Not {
    fn name(&self) -> &'static str {
        "Style/Not"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["unary"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            report_not(self, source, child, diagnostics, &mut corrections);
        }
    }
}

fn report_not(
    cop: &Not,
    source: &SourceFile,
    child: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let text = &source.as_bytes()[child.start_byte()..child.end_byte()];
    if text != b"not" {
        return;
    }
    let (line, col) = source.offset_to_line_col(child.start_byte());
    let mut diag = cop.diagnostic(source, line, col, "Use `!` instead of `not`.".to_string());
    push_bang(cop, child, corrections, &mut diag);
    diagnostics.push(diag);
}

fn push_bang(
    cop: &Not,
    child: Node<'_>,
    corrections: &mut Option<&mut Vec<Correction>>,
    diag: &mut Diagnostic,
) {
    let Some(corr) = corrections else {
        return;
    };
    corr.push(Correction {
        start: child.start_byte(),
        end: child.end_byte(),
        replacement: "!".into(),
        cop_name: cop.name(),
        cop_index: 0,
        owner: None,
    });
    diag.corrected = true;
}
