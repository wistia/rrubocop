//! Prefer Active Support's blank? predicate for equivalent condition shapes.

use tree_sitter::Node;

use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, collect_comments, node_text, push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct Blank;

impl Cop for Blank {
    fn name(&self) -> &'static str {
        "Rails/Blank"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn safe_autocorrect(&self) -> bool {
        false
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["binary", "unary", "unless", "unless_modifier"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if matches!(node.kind(), "unless" | "unless_modifier") {
            check_unless(self, source, node, config, diagnostics, &mut corrections);
            return;
        }
        let receiver = match node.kind() {
            "binary" if config.get_bool("NilOrEmpty", true) => nil_or_empty(source, node),
            "unary" if config.get_bool("NotPresent", true) && !defines_blank(source, node) => {
                negated(node, source).and_then(|n| predicate(source, n, b"present?"))
            }
            _ => return,
        };
        let Some(receiver) = receiver else {
            return;
        };
        let replacement = receiver
            .map(|n| format!("{}.blank?", node_text(source, n)))
            .unwrap_or_else(|| "blank?".into());
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            col,
            format!(
                "Use `{replacement}` instead of `{}`.",
                node_text(source, node)
            ),
        );
        diagnostic.highlight_length = highlight_length(source, node.start_byte(), node.end_byte());
        if collect_comments(node).is_empty() {
            diagnostic.corrected =
                replace_with_predicate(self, node, receiver, "blank?", &mut corrections);
        } else {
            diagnostic.correctable = false;
        }
        diagnostic.correctable &= !config.autocorrect_disabled();
        diagnostics.push(diagnostic);
    }
}

pub(super) fn predicate<'a>(
    source: &SourceFile,
    node: Node<'a>,
    name: &[u8],
) -> Option<Option<Node<'a>>> {
    if node
        .child_by_field_name("operator")
        .is_some_and(|op| node_text(source, op) == "&.")
    {
        return None;
    }
    if call_method_name(source, node) == Some(name)
        && argument_nodes(node).is_empty()
        && node.child_by_field_name("block").is_none()
    {
        Some(call_receiver(node))
    } else if node.kind() == "identifier" && source.as_bytes()[node.byte_range()] == *name {
        Some(None)
    } else {
        None
    }
}

pub(super) fn negated<'a>(node: Node<'a>, source: &SourceFile) -> Option<Node<'a>> {
    if node.kind() == "unary"
        && node
            .child_by_field_name("operator")
            .is_some_and(|op| matches!(node_text(source, op).as_str(), "!" | "not"))
    {
        node.child_by_field_name("operand")
    } else {
        None
    }
}

fn nil_receiver<'a>(source: &SourceFile, node: Node<'a>) -> Option<Option<Node<'a>>> {
    if let Some(value) = predicate(source, node, b"nil?") {
        return Some(value);
    }
    if let Some(value) = negated(node, source) {
        return Some(Some(value));
    }
    if node.kind() != "binary"
        || node
            .child_by_field_name("operator")
            .is_none_or(|op| node_text(source, op) != "==")
    {
        return None;
    }
    let left = node.child_by_field_name("left")?;
    let right = node.child_by_field_name("right")?;
    if left.kind() == "nil" {
        Some(Some(right))
    } else if right.kind() == "nil" {
        Some(Some(left))
    } else {
        None
    }
}

fn nil_or_empty<'a>(source: &SourceFile, node: Node<'a>) -> Option<Option<Node<'a>>> {
    let op = node.child_by_field_name("operator")?;
    if !matches!(node_text(source, op).as_str(), "||" | "or") {
        return None;
    }
    let left = nil_receiver(source, node.child_by_field_name("left")?)?;
    let right = node.child_by_field_name("right")?;
    let right = negated(right, source)
        .and_then(|n| negated(n, source))
        .unwrap_or(right);
    let right = predicate(source, right, b"empty?")?;
    (left.map(|n| &source.as_bytes()[n.byte_range()])
        == right.map(|n| &source.as_bytes()[n.byte_range()]))
    .then_some(left)
}

fn defines_blank(source: &SourceFile, node: Node<'_>) -> bool {
    let Some(body) = node.parent() else {
        return false;
    };
    let mut cursor = body.walk();
    if body.kind() != "body_statement"
        || body
            .named_children(&mut cursor)
            .filter(|n| n.kind() != "comment")
            .count()
            != 1
    {
        return false;
    }
    body.parent()
        .filter(|method| method.kind() == "method")
        .and_then(|method| method.child_by_field_name("name"))
        .is_some_and(|name| node_text(source, name) == "blank?")
}

pub(super) fn highlight_length(source: &SourceFile, start: usize, end: usize) -> usize {
    let bytes = &source.as_bytes()[start..end];
    let length = bytes
        .iter()
        .position(|byte| *byte == b'\n')
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..length]).chars().count()
}

pub(super) fn replace_with_predicate(
    cop: &dyn Cop,
    node: Node<'_>,
    receiver: Option<Node<'_>>,
    method: &str,
    corrections: &mut Option<&mut Vec<Correction>>,
) -> bool {
    if let Some(receiver) = receiver {
        if node.start_byte() != receiver.start_byte() {
            push_replace(
                corrections,
                node.start_byte(),
                receiver.start_byte(),
                "",
                cop.name(),
            );
        }
        push_replace(
            corrections,
            receiver.end_byte(),
            node.end_byte(),
            format!(".{method}"),
            cop.name(),
        )
    } else {
        push_replace(
            corrections,
            node.start_byte(),
            node.end_byte(),
            method,
            cop.name(),
        )
    }
}

fn check_unless(
    cop: &Blank,
    source: &SourceFile,
    node: Node<'_>,
    config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    if !config.get_bool("UnlessPresent", true) || node.child_by_field_name("alternative").is_some()
    {
        return;
    }
    let Some(condition) = node.child_by_field_name("condition") else {
        return;
    };
    let Some(receiver) = predicate(source, condition, b"present?") else {
        return;
    };
    let mut cursor = node.walk();
    let Some(keyword) = node.children(&mut cursor).find(|n| n.kind() == "unless") else {
        return;
    };
    let replacement = receiver
        .map(|n| format!("{}.blank?", node_text(source, n)))
        .unwrap_or_else(|| "blank?".into());
    let (line, col) = source.offset_to_line_col(keyword.start_byte());
    let original = &source.as_bytes()[keyword.start_byte()..condition.end_byte()];
    let mut diagnostic = cop.diagnostic(
        source,
        line,
        col,
        format!(
            "Use `if {replacement}` instead of `{}`.",
            String::from_utf8_lossy(original)
        ),
    );
    diagnostic.highlight_length =
        highlight_length(source, keyword.start_byte(), condition.end_byte());
    diagnostic.corrected = replace_with_predicate(cop, condition, receiver, "blank?", corrections);
    push_replace(
        corrections,
        keyword.start_byte(),
        keyword.end_byte(),
        "if",
        cop.name(),
    );
    diagnostic.correctable &= !config.autocorrect_disabled();
    diagnostics.push(diagnostic);
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Blank, "cops/rails/blank");

    #[test]
    fn preserves_raw_receiver_bytes() {
        let before = b"\"\xff\".nil? || \"\xff\".empty?\n";
        let source = SourceFile::from_bytes("test.rb", before.to_vec());
        let tree = crate::parse::parse_ruby(&source).unwrap();
        let config = CopConfig::default();
        let mut diagnostics = vec![];
        let mut edits = vec![];
        crate::cop::walker::BatchedWalker::new(vec![&Blank], vec![&config]).walk(
            &source,
            tree.root_node(),
            &mut diagnostics,
            Some(&mut edits),
        );
        assert_eq!(
            crate::correction::CorrectionSet::from_vec(edits).apply(before),
            b"\"\xff\".blank?\n"
        );
        let different =
            crate::testutil::run_cop_full(&Blank, b"\"\xff\".nil? || \"\xfe\".empty?\n");
        assert!(different.is_empty());
    }

    #[test]
    fn does_not_delete_comments_between_predicates() {
        let bytes = b"items.nil? || # keep this\n items.empty?\n";
        let source = SourceFile::from_bytes("test.rb", bytes.to_vec());
        let tree = crate::parse::parse_ruby(&source).unwrap();
        let config = CopConfig::default();
        let mut diagnostics = vec![];
        let mut edits = vec![];
        crate::cop::walker::BatchedWalker::new(vec![&Blank], vec![&config]).walk(
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
