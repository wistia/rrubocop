use tree_sitter::Node;

use crate::cop::shared::node_text;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Lint/RedundantRegexpQuantifiers — nested redundant quantifiers like `a**`.
pub struct RedundantRegexpQuantifiers;

fn is_quant(b: u8) -> bool {
    matches!(b, b'*' | b'+' | b'?')
}

fn skip_valid_pair(a: u8, b: u8) -> bool {
    // reluctant `*?` `+?` or leading `?`
    (b == b'?' && a != b'?') || a == b'?'
}

fn combined_quant(a: u8, b: u8) -> char {
    if a == b'*' || b == b'*' {
        '*'
    } else if a == b'+' || b == b'+' {
        '+'
    } else {
        '?'
    }
}

fn redundant_pair(a: u8, b: u8) -> bool {
    is_quant(a) && is_quant(b) && !skip_valid_pair(a, b)
}

fn find_redundant(pattern: &str) -> Option<(char, char, char)> {
    let bytes = pattern.as_bytes();
    let mut in_class = false;
    let mut escaped = false;
    for i in 0..bytes.len().saturating_sub(1) {
        if escaped {
            escaped = false;
            continue;
        }
        if bytes[i] == b'\\' {
            escaped = true;
            continue;
        }
        if bytes[i] == b'[' {
            in_class = true;
        }
        if bytes[i] == b']' {
            in_class = false;
        }
        if in_class {
            continue;
        }
        let (a, b) = (bytes[i], bytes[i + 1]);
        if !redundant_pair(a, b) {
            continue;
        }
        return Some((a as char, b as char, combined_quant(a, b)));
    }
    None
}

impl Cop for RedundantRegexpQuantifiers {
    fn name(&self) -> &'static str {
        "Lint/RedundantRegexpQuantifiers"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["regex"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let Some((inner_q, outer_q, combined)) = find_redundant(
            node_text(source, node)
                .trim_start_matches('/')
                .trim_end_matches(|c| matches!(c, '/' | 'i' | 'm' | 'x' | 'o')),
        ) else {
            return;
        };
        let (line, col) = source.offset_to_line_col(node.start_byte());
        diagnostics.push(self.diagnostic(
            source,
            line,
            col,
            format!(
                "Replace redundant quantifiers `{inner_q}` and `{outer_q}` with a single `{combined}`."
            ),
        ));
    }
}
