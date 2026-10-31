//! Style/SymbolLiteral — avoid quoted symbols when unnecessary.

use tree_sitter::Node;

use crate::cop::shared::node_bytes;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct SymbolLiteral;

impl Cop for SymbolLiteral {
    fn name(&self) -> &'static str {
        "Style/SymbolLiteral"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["simple_symbol", "symbol", "hash_key_symbol"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let b = node_bytes(source, node);
        if !(b.starts_with(b":\"") || b.starts_with(b":'")) {
            return;
        }
        report(self, source, node, b, diagnostics, &mut corrections);
    }
}

fn report(
    cop: &SymbolLiteral,
    source: &SourceFile,
    node: Node<'_>,
    b: &[u8],
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (line, col) = source.offset_to_line_col(node.start_byte());
    let mut diag = cop.diagnostic(
        source,
        line,
        col,
        "Do not quote symbols unnecessarily.".to_string(),
    );
    if is_word_like_quoted(b) {
        push_unquote(cop, node, b, corrections, &mut diag);
    }
    diagnostics.push(diag);
}

fn push_unquote(
    cop: &SymbolLiteral,
    node: Node<'_>,
    b: &[u8],
    corrections: &mut Option<&mut Vec<Correction>>,
    diag: &mut Diagnostic,
) {
    let Some(corr) = corrections.as_mut() else {
        return;
    };
    corr.push(Correction {
        start: node.start_byte(),
        end: node.end_byte(),
        replacement: b
            .iter()
            .copied()
            .filter(|&c| c != b'\'' && c != b'"')
            .map(|c| c as char)
            .collect(),
        cop_name: cop.name(),
        cop_index: 0,
        owner: None,
    });
    diag.corrected = true;
}

fn is_word_like_quoted(b: &[u8]) -> bool {
    quoted_inner(b).is_some_and(is_word_ident)
}

fn quoted_inner(b: &[u8]) -> Option<&[u8]> {
    if b.len() < 4 || b[0] != b':' {
        return None;
    }
    let q = b[1];
    if q != b'\'' && q != b'"' {
        return None;
    }
    if *b.last()? != q {
        return None;
    }
    Some(&b[2..b.len() - 1])
}

fn is_word_ident(inner: &[u8]) -> bool {
    let Some(&first) = inner.first() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return false;
    }
    inner[1..]
        .iter()
        .all(|c| c.is_ascii_alphanumeric() || *c == b'_')
}
