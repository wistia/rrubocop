//! Style/StringLiterals — breadth-first tree-sitter port.

use tree_sitter::Node;

use crate::cop::style::heuristics::matches_string_literals;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct StringLiterals;

impl Cop for StringLiterals {
    fn name(&self) -> &'static str {
        "Style/StringLiterals"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["string", "string_content"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if !matches_string_literals(source, node, config) {
            return;
        }
        let style = config.get_str("EnforcedStyle", "single_quotes");
        let msg = if style == "double_quotes" {
            "Prefer double-quoted strings unless you need single quotes to avoid extra backslashes for escaping."
        } else {
            "Prefer single-quoted strings when you don't need string interpolation or special symbols."
        };
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut diag = self.diagnostic(source, line, col, msg.to_string());
        push_fix(self, source, node, style, &mut corrections, &mut diag);
        diagnostics.push(diag);
    }
}

fn push_fix(
    cop: &StringLiterals,
    source: &SourceFile,
    node: Node<'_>,
    style: &str,
    corrections: &mut Option<&mut Vec<Correction>>,
    diag: &mut Diagnostic,
) {
    let Some(corr) = corrections.as_mut() else {
        return;
    };
    let bytes = &source.as_bytes()[node.start_byte()..node.end_byte()];
    let Some(replacement) = rewrite_quotes(bytes, style) else {
        return;
    };
    corr.push(Correction {
        start: node.start_byte(),
        end: node.end_byte(),
        replacement,
        cop_name: cop.name(),
        cop_index: 0,
        owner: None,
    });
    diag.corrected = true;
}

fn rewrite_quotes(bytes: &[u8], style: &str) -> Option<String> {
    let open = *bytes.first()?;
    if bytes.len() < 2 || bytes.last().copied() != Some(open) {
        return None;
    }
    let inner = &bytes[1..bytes.len() - 1];
    match (style, open) {
        ("single_quotes", b'"') => Some(to_single(inner)),
        ("double_quotes", b'\'') => Some(to_double(inner)),
        _ => None,
    }
}

fn to_single(inner: &[u8]) -> String {
    let content = unescape_double(inner);
    let mut out = Vec::with_capacity(content.len() + 2);
    out.push(b'\'');
    for &b in &content {
        if b == b'\\' {
            out.extend_from_slice(b"\\\\");
        } else {
            out.push(b);
        }
    }
    out.push(b'\'');
    String::from_utf8(out).unwrap_or_else(|e| String::from_utf8_lossy(&e.into_bytes()).into_owned())
}

fn to_double(inner: &[u8]) -> String {
    let content = unescape_single(inner);
    let mut out = Vec::with_capacity(content.len() + 2);
    out.push(b'"');
    for &b in &content {
        match b {
            b'\\' => out.extend_from_slice(b"\\\\"),
            b'"' => out.extend_from_slice(b"\\\""),
            b => out.push(b),
        }
    }
    out.push(b'"');
    String::from_utf8(out).unwrap_or_else(|e| String::from_utf8_lossy(&e.into_bytes()).into_owned())
}

fn unescape_double(inner: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(inner.len());
    let mut i = 0;
    while i < inner.len() {
        if inner[i] == b'\\' && i + 1 < inner.len() {
            out.push(inner[i + 1]);
            i += 2;
        } else {
            out.push(inner[i]);
            i += 1;
        }
    }
    out
}

fn unescape_single(inner: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(inner.len());
    let mut i = 0;
    while i < inner.len() {
        if inner[i] == b'\\' && i + 1 < inner.len() && matches!(inner[i + 1], b'\\' | b'\'') {
            out.push(inner[i + 1]);
            i += 2;
        } else {
            out.push(inner[i]);
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(StringLiterals, "cops/style/string_literals");

    #[test]
    fn rewrite_preserves_utf8() {
        assert_eq!(
            rewrite_quotes("\"café\"".as_bytes(), "single_quotes").as_deref(),
            Some("'café'")
        );
        assert_eq!(
            rewrite_quotes("'café'".as_bytes(), "double_quotes").as_deref(),
            Some("\"café\"")
        );
    }
}
