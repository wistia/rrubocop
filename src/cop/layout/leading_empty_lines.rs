//! Layout/LeadingEmptyLines: locate the original lexer's first visible token.
use crate::cop::shared::{for_each_descendant, node_bytes, push_replace};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use tree_sitter::{Node, Tree};

pub struct LeadingEmptyLines;

fn first_token<'a>(source: &SourceFile, mut node: Node<'a>) -> Option<Node<'a>> {
    loop {
        if node.kind() == "uninterpreted" {
            return None;
        }
        if matches!(node.kind(), "integer" | "float" | "rational" | "complex") {
            return Some(node);
        }
        if node.kind() == "string"
            && node_bytes(source, node)
                .first()
                .is_some_and(|b| matches!(b, b'\'' | b'"'))
        {
            let mut dynamic = false;
            for_each_descendant(node, |n| dynamic |= n.kind() == "interpolation");
            let mut escaped = false;
            let mut newline = false;
            for byte in node_bytes(source, node) {
                if *byte == b'\n' && !escaped {
                    newline = true;
                }
                if *byte == b'\\' {
                    escaped = !escaped;
                } else if *byte != b'\r' {
                    escaped = false;
                }
            }
            if !dynamic && !newline {
                return Some(node);
            }
        }
        if node.child_count() == 0 {
            return (node.kind() != "program" && !node.is_missing()).then_some(node);
        }
        node = node.child(0)?;
    }
}

impl Cop for LeadingEmptyLines {
    fn name(&self) -> &'static str {
        "Layout/LeadingEmptyLines"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn uses_source_phase(&self) -> bool {
        true
    }
    fn check_source(
        &self,
        source: &SourceFile,
        tree: &Tree,
        _map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(token) = first_token(source, tree.root_node()) else {
            return;
        };
        let (line, column) = source.offset_to_line_col(token.start_byte());
        if line <= 1 {
            return;
        }
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "Unnecessary blank line at the beginning of the source.".into(),
        );
        let bytes = source.as_bytes();
        let mut end = token.end_byte();
        let block_comment =
            token.kind() == "comment" && node_bytes(source, token).starts_with(b"=begin");
        if block_comment {
            if bytes[end..].starts_with(b"\r\n") {
                end += 2;
            } else if bytes[end..].starts_with(b"\n") {
                end += 1;
            }
        }
        diagnostic.highlight_length = String::from_utf8_lossy(&bytes[token.start_byte()..end])
            .replace("\r\n", "\n")
            .chars()
            .count();
        if block_comment && end == token.end_byte() && end == bytes.len() {
            diagnostic.highlight_length += 2;
        }
        diagnostic.corrected =
            push_replace(&mut corrections, 0, token.start_byte(), "", self.name());
        for (offset, bytes) in source.as_bytes()[token.start_byte()..]
            .windows(2)
            .enumerate()
        {
            if bytes == b"\r\n" {
                let start = token.start_byte() + offset;
                push_replace(&mut corrections, start, start + 1, "", self.name());
            }
        }
        diagnostics.push(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_ruby_states() {
        crate::testutil::assert_original_spec_parity(
            &LeadingEmptyLines,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/layout/leading_empty_lines/original-parity.json"
            )),
        );
    }
}
