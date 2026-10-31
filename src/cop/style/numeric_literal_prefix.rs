//! Style/NumericLiteralPrefix — 0o / 0d / 0b prefixes.

use tree_sitter::Node;

use crate::cop::shared::node_bytes;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct NumericLiteralPrefix;

impl Cop for NumericLiteralPrefix {
    fn name(&self) -> &'static str {
        "Style/NumericLiteralPrefix"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["integer"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if !needs_octal_prefix(node_bytes(source, node)) {
            return;
        }
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut diag = self.diagnostic(
            source,
            line,
            col,
            "Use `0o` for octal literals.".to_string(),
        );
        if let Some(corr) = corrections.as_mut() {
            // 0123 → 0o123 (insert `o` after leading `0`)
            corr.push(Correction {
                start: node.start_byte() + 1,
                end: node.start_byte() + 1,
                replacement: "o".into(),
                cop_name: self.name(),
                cop_index: 0,
                owner: None,
            });
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}

fn needs_octal_prefix(text: &[u8]) -> bool {
    text.len() >= 2
        && text[0] == b'0'
        && text[1].is_ascii_digit()
        && !text[1..]
            .iter()
            .any(|b| matches!(b, b'b' | b'o' | b'x' | b'd'))
}
