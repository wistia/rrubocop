//! Reuse the default URI parser when no constructor options are supplied.
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, collect_comments, node_text, push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct UriDefaultParser;
impl Cop for UriDefaultParser {
    fn name(&self) -> &'static str {
        "Performance/UriDefaultParser"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"new"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if call_method_name(source, node) != Some(b"new") || !argument_nodes(node).is_empty() {
            return;
        }
        let Some(receiver) = call_receiver(node) else {
            return;
        };
        let value = node_text(source, receiver)
            .split_whitespace()
            .collect::<String>();
        if !matches!(value.as_str(), "URI::Parser" | "::URI::Parser") {
            return;
        }
        if node
            .child_by_field_name("operator")
            .is_some_and(|n| node_text(source, n) == "&.")
        {
            return;
        }
        let prefix = if value.starts_with("::") { "::" } else { "" };
        let replacement = format!("{prefix}URI::DEFAULT_PARSER");
        let end = super::send_range::end(node);
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            format!("Use `{replacement}` instead of `{prefix}URI::Parser.new`."),
        );
        let raw = &source.as_bytes()[node.start_byte()..end];
        let line_end = raw
            .iter()
            .position(|byte| *byte == b'\n')
            .unwrap_or(raw.len());
        d.highlight_length = String::from_utf8_lossy(&raw[..line_end]).chars().count();
        d.correctable = !config.autocorrect_disabled()
            && !collect_comments(node)
                .iter()
                .any(|comment| comment.start_byte() < end);
        if d.correctable {
            d.corrected = push_replace(
                &mut corrections,
                node.start_byte(),
                end,
                replacement,
                self.name(),
            );
        }
        diagnostics.push(d);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UriDefaultParser, "cops/performance/uri_default_parser");

    #[test]
    fn does_not_discard_comments_in_empty_constructor_parentheses() {
        let before = b"URI::Parser.new( # keep this\n)\n";
        let source = SourceFile::from_bytes("test.rb", before.to_vec());
        let tree = crate::parse::parse_ruby(&source).unwrap();
        let config = CopConfig::default();
        let mut diagnostics = vec![];
        let mut edits = vec![];
        crate::cop::walker::BatchedWalker::new(vec![&UriDefaultParser], vec![&config]).walk(
            &source,
            tree.root_node(),
            &mut diagnostics,
            Some(&mut edits),
        );
        assert!(edits.is_empty());
        assert_eq!(diagnostics.len(), 1);
        assert!(!diagnostics[0].correctable);
    }
}
