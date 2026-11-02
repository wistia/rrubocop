//! Combine repeated string prefix or suffix predicates when later arguments are pure.
use crate::cop::shared::ruby_expression::same_expression;
use crate::cop::shared::ruby_semantics::pure;
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, collect_comments, node_bytes, node_text,
    push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct DoubleStartEndWith;

impl Cop for DoubleStartEndWith {
    fn name(&self) -> &'static str {
        "Performance/DoubleStartEndWith"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["binary"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if !node
            .child_by_field_name("operator")
            .is_some_and(|n| matches!(node_bytes(source, n), b"||" | b"or"))
        {
            return;
        }
        let (Some(left), Some(right)) = (
            node.child_by_field_name("left"),
            node.child_by_field_name("right"),
        ) else {
            return;
        };
        if left.kind() != "call"
            || right.kind() != "call"
            || [left, right].iter().any(|n| {
                n.child_by_field_name("block").is_some()
                    || n.child_by_field_name("operator")
                        .is_some_and(|n| node_bytes(source, n) == b"&.")
            })
        {
            return;
        }
        let Some(method) = call_method_name(source, left) else {
            return;
        };
        if method != call_method_name(source, right).unwrap_or_default()
            || !(matches!(method, b"start_with?" | b"end_with?")
                || config.get_bool("IncludeActiveSupportAliases", false)
                    && matches!(method, b"starts_with?" | b"ends_with?"))
        {
            return;
        }
        let (Some(receiver), Some(other)) = (call_receiver(left), call_receiver(right)) else {
            return;
        };
        if !same_expression(source, receiver, other) {
            return;
        }
        let first = argument_nodes(left);
        let second = argument_nodes(right);
        if first.is_empty() || second.is_empty() || !second.iter().all(|n| pure(source, *n)) {
            return;
        }
        let args = first
            .iter()
            .chain(&second)
            .map(|n| node_text(source, *n))
            .collect::<Vec<_>>()
            .join(", ");
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            format!(
                "Use `{}.{}({args})` instead of `{}`.",
                node_text(source, receiver),
                String::from_utf8_lossy(method),
                node_text(source, node)
            ),
        );
        let raw = node_bytes(source, node);
        d.highlight_length =
            String::from_utf8_lossy(raw.split(|b| *b == b'\n').next().unwrap_or(raw))
                .chars()
                .count();
        let start = first[0].start_byte();
        let end = second.last().unwrap().end_byte();
        d.correctable = !config.autocorrect_disabled()
            && pure(source, receiver)
            && std::str::from_utf8(&source.as_bytes()[start..end]).is_ok()
            && !collect_comments(node)
                .iter()
                .any(|c| c.start_byte() >= start && c.start_byte() < end);
        if d.correctable {
            d.corrected = push_replace(&mut corrections, start, end, args, self.name());
        }
        diagnostics.push(d);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(DoubleStartEndWith, "cops/performance/double_start_end_with");
}
