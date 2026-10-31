//! Style/CharacterLiteral — avoid ?x character literals.

use tree_sitter::Node;

use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct CharacterLiteral;

impl Cop for CharacterLiteral {
    fn name(&self) -> &'static str {
        "Style/CharacterLiteral"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["character"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        report(self, source, node, diagnostics, &mut corrections);
    }
}

fn report(
    cop: &CharacterLiteral,
    source: &SourceFile,
    node: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (line, col) = source.offset_to_line_col(node.start_byte());
    let mut diag = cop.diagnostic(
        source,
        line,
        col,
        "Do not use the character literal - use string literal instead.".to_string(),
    );
    if let Some(corr) = corrections.as_mut() {
        corr.push(Correction {
            start: node.start_byte(),
            end: node.end_byte(),
            replacement: to_string_lit(source, node),
            cop_name: cop.name(),
            cop_index: 0,
            owner: None,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

fn to_string_lit(source: &SourceFile, node: Node<'_>) -> String {
    let bytes = &source.as_bytes()[node.start_byte()..node.end_byte()];
    if bytes.len() == 2
        && bytes[0] == b'?'
        && bytes[1].is_ascii_graphic()
        && bytes[1] != b'\\'
        && bytes[1] != b'"'
    {
        return format!("\"{}\"", bytes[1] as char);
    }
    if bytes.first() == Some(&b'?') {
        let src = std::str::from_utf8(&bytes[1..]).unwrap_or("?");
        return format!("\"{src}\"");
    }
    format!("\"{}\"", String::from_utf8_lossy(bytes))
}
