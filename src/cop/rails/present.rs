//! Prefer the positive Active Support presence predicate.
use super::blank::{highlight_length, negated, predicate, replace_with_predicate};
use crate::cop::shared::{collect_comments, node_text, push_replace};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct Present;
impl Cop for Present {
    fn name(&self) -> &'static str {
        "Rails/Present"
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
        let mut keyword = None;
        let receiver = match node.kind() {
            "binary" if config.get_bool("NotNilAndNotEmpty", true) => {
                exists_and_not_empty(source, node)
            }
            "unary" if config.get_bool("NotBlank", true) => {
                negated(node, source).and_then(|n| predicate(source, n, b"blank?"))
            }
            "unless" | "unless_modifier"
                if config.get_bool("UnlessBlank", true)
                    && node.child_by_field_name("alternative").is_none() =>
            {
                let Some(condition) = node.child_by_field_name("condition") else {
                    return;
                };
                let mut cursor = node.walk();
                keyword = node.children(&mut cursor).find(|n| n.kind() == "unless");
                predicate(source, condition, b"blank?")
            }
            _ => return,
        };
        let Some(receiver) = receiver else {
            return;
        };
        let replacement = receiver
            .map(|n| format!("{}.present?", node_text(source, n)))
            .unwrap_or_else(|| "present?".into());
        let start = keyword.map_or(node.start_byte(), |n| n.start_byte());
        let condition = node.child_by_field_name("condition").unwrap_or(node);
        let end = if keyword.is_some() {
            condition.end_byte()
        } else {
            node.end_byte()
        };
        let current = String::from_utf8_lossy(&source.as_bytes()[start..end]);
        let prefer = if keyword.is_some() {
            format!("if {replacement}")
        } else {
            replacement.clone()
        };
        let (line, col) = source.offset_to_line_col(start);
        let mut d = self.diagnostic(
            source,
            line,
            col,
            format!("Use `{prefer}` instead of `{current}`."),
        );
        d.highlight_length = highlight_length(source, start, end);
        if !collect_comments(node).is_empty() && keyword.is_none() {
            d.correctable = false;
        } else if let Some(keyword) = keyword {
            d.corrected = push_replace(
                &mut corrections,
                keyword.start_byte(),
                keyword.end_byte(),
                "if",
                self.name(),
            );
            replace_with_predicate(self, condition, receiver, "present?", &mut corrections);
        } else {
            d.corrected =
                replace_with_predicate(self, node, receiver, "present?", &mut corrections);
        }
        d.correctable &= !config.autocorrect_disabled();
        diagnostics.push(d);
    }
}
fn exists_and_not_empty<'a>(source: &SourceFile, node: Node<'a>) -> Option<Option<Node<'a>>> {
    if node
        .child_by_field_name("operator")
        .is_none_or(|op| !matches!(node_text(source, op).as_str(), "&&" | "and"))
    {
        return None;
    }
    let left = node.child_by_field_name("left")?;
    let right = predicate(
        source,
        negated(node.child_by_field_name("right")?, source)?,
        b"empty?",
    )?;
    let left = if let Some(negation) = negated(left, source) {
        if let Some(receiver) = predicate(source, negation, b"nil?") {
            receiver
        } else if let Some(receiver) = negated(negation, source) {
            Some(receiver)
        } else {
            Some(left)
        }
    } else if left.kind() == "binary"
        && left
            .child_by_field_name("operator")
            .is_some_and(|op| node_text(source, op) == "!=")
        && left
            .child_by_field_name("right")
            .is_some_and(|n| n.kind() == "nil")
    {
        left.child_by_field_name("left")
    } else {
        Some(left)
    };
    (left.map(|n| &source.as_bytes()[n.byte_range()])
        == right.map(|n| &source.as_bytes()[n.byte_range()]))
    .then_some(left)
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Present, "cops/rails/present");
    #[test]
    fn requires_explicit_unsafe_correction_for_value_changing_rewrites() {
        assert!(!Present.safe_autocorrect());
    }
}
