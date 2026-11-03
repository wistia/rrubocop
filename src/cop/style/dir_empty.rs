//! Style/DirEmpty.

use tree_sitter::Node;

use crate::cop::shared::{
    call_method_name, call_receiver, integer_value, node_bytes, push_replace,
    semantic_argument_nodes as argument_nodes,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct DirEmpty;

fn ordinary_call(source: &SourceFile, node: Node<'_>) -> bool {
    node.kind() == "call"
        && !node
            .child_by_field_name("operator")
            .is_some_and(|operator| node_bytes(source, operator) == b"&.")
}

fn empty_check<'a>(source: &SourceFile, node: Node<'a>) -> Option<(Node<'a>, bool, bool)> {
    if node.kind() == "binary" {
        let operator = node.child_by_field_name("operator")?;
        let name = node_bytes(source, operator);
        if !matches!(name, b"==" | b"!=" | b">") {
            return None;
        }
        let size = node.child_by_field_name("left")?;
        if !ordinary_call(source, size)
            || call_method_name(source, size) != Some(b"size")
            || size.child_by_field_name("block").is_some()
            || !argument_nodes(size).is_empty()
        {
            return None;
        }
        let count = node.child_by_field_name("right")?;
        let value = integer_value(source, count)?;
        if value != 0.into() && value != 2.into() {
            return None;
        }
        Some((call_receiver(size)?, name != b"==", value == 2.into()))
    } else if ordinary_call(source, node) && argument_nodes(node).is_empty() {
        let method = call_method_name(source, node)?;
        let inner = call_receiver(node)?;
        match (method, call_method_name(source, inner)) {
            (b"empty?", Some(b"children")) | (b"none?", Some(b"each_child")) => {
                Some((inner, false, false))
            }
            _ => None,
        }
    } else {
        None
    }
}

impl Cop for DirEmpty {
    fn name(&self) -> &'static str {
        "Style/DirEmpty"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "binary"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if config.get_f64("TargetRubyVersion", 3.4) < 2.4 {
            return;
        }
        let Some((inner, negate, entries)) = empty_check(source, node) else {
            return;
        };
        if !ordinary_call(source, inner) || inner.child_by_field_name("block").is_some() {
            return;
        }
        let Some(receiver) = call_receiver(inner) else {
            return;
        };
        if !matches!(node_bytes(source, receiver), b"Dir" | b"::Dir") {
            return;
        }
        let arguments = argument_nodes(inner);
        if arguments.len() != 1 {
            return;
        }
        if node.kind() == "binary"
            && call_method_name(source, inner)
                != Some(if entries { b"entries" } else { b"children" })
        {
            return;
        }
        let argument = arguments[0];
        let constant = String::from_utf8_lossy(node_bytes(source, receiver));
        let prefix = format!("{}{constant}.empty?(", if negate { "!" } else { "" });
        let replacement = format!(
            "{prefix}{})",
            String::from_utf8_lossy(node_bytes(source, argument))
        );
        let (line, column) = source.offset_to_line_col(node.start_byte());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            format!("Use `{replacement}` instead."),
        );
        let mut end = node
            .child_by_field_name("block")
            .map_or(node.end_byte(), |block| block.start_byte());
        while end > node.start_byte() && source.as_bytes()[end - 1].is_ascii_whitespace() {
            end -= 1;
        }
        diagnostic.highlight_length =
            String::from_utf8_lossy(&source.as_bytes()[node.start_byte()..end])
                .chars()
                .count();
        let comments = crate::cop::shared::collect_comments(node);
        let deletes_comment = node.child_by_field_name("block").is_some()
            || comments.iter().any(|comment| {
                comment.start_byte() < argument.start_byte()
                    || comment.end_byte() > argument.end_byte()
            });
        if deletes_comment {
            diagnostic.correctable = false;
        } else {
            diagnostic.corrected = push_replace(
                &mut corrections,
                node.start_byte(),
                argument.start_byte(),
                prefix,
                self.name(),
            );
            push_replace(
                &mut corrections,
                argument.end_byte(),
                node.end_byte(),
                ")",
                self.name(),
            );
        }
        diagnostics.push(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_comments_outside_the_retained_argument() {
        let source =
            SourceFile::from_bytes("test.rb", b"Dir.children( # keep\n path).empty?\n".to_vec());
        let tree = crate::parse::parse_ruby(&source).unwrap();
        let mut diagnostics = Vec::new();
        let mut corrections = Vec::new();
        crate::cop::shared::for_each_descendant(tree.root_node(), |node| {
            DirEmpty.check_node(
                &source,
                node,
                &CopConfig::default(),
                &mut diagnostics,
                Some(&mut corrections),
            );
        });
        assert_eq!(diagnostics.len(), 1);
        assert!(!diagnostics[0].correctable);
        assert!(corrections.is_empty());
    }

    #[test]
    fn preserves_raw_argument_bytes() {
        let original = b"# encoding: ASCII-8BIT\nDir.children('\xff').empty?\n";
        let source = SourceFile::from_bytes("test.rb", original.to_vec());
        let tree = crate::parse::parse_ruby(&source).unwrap();
        let mut diagnostics = Vec::new();
        let mut corrections = Vec::new();
        crate::cop::shared::for_each_descendant(tree.root_node(), |node| {
            DirEmpty.check_node(
                &source,
                node,
                &CopConfig::default(),
                &mut diagnostics,
                Some(&mut corrections),
            );
        });
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            crate::correction::CorrectionSet::from_vec(corrections).apply(original),
            b"# encoding: ASCII-8BIT\nDir.empty?('\xff')\n"
        );
    }

    #[test]
    fn original_ruby_specs() {
        crate::testutil::assert_original_spec_parity(
            &DirEmpty,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/dir_empty/parity.json"
            )),
        );
    }
}
