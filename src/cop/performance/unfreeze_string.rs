//! Prefer unary plus when constructing mutable string literals.
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, collect_comments, node_text, push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct UnfreezeString;
impl Cop for UnfreezeString {
    fn name(&self) -> &'static str {
        "Performance/UnfreezeString"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn safe_autocorrect(&self) -> bool {
        false
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"dup", b"new"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if crate::cop::shared::is_setter_call(node) {
            return;
        }
        let version = config.get_f64("TargetRubyVersion", 3.4);
        if version < 2.3
            || node
                .child_by_field_name("operator")
                .is_some_and(|op| node_text(source, op) == "&.")
        {
            return;
        }
        let Some(receiver) = call_receiver(node) else {
            return;
        };
        let args = argument_nodes(node);
        let literal = |n: Node<'_>| matches!(n.kind(), "string" | "heredoc_beginning");
        let value = match call_method_name(source, node) {
            Some(b"dup") if version <= 3.2 && literal(receiver) && args.is_empty() => {
                Some(receiver)
            }
            Some(b"new")
                if receiver.kind() == "constant" && node_text(source, receiver) == "String" =>
            {
                match args.as_slice() {
                    [] => None,
                    [arg] if literal(*arg) => Some(*arg),
                    _ => return,
                }
            }
            _ => return,
        };
        let end = super::send_range::end(node);
        let raw = &source.as_bytes()[node.start_byte()..end];
        let mut replacement = String::from("+");
        replacement.push_str(&value.map_or_else(|| "''".into(), |n| node_text(source, n)));
        if node.parent().is_some_and(|parent| {
            parent.kind() == "call"
                || parent.kind() == "element_reference"
                || parent.kind() == "binary"
                || parent.kind() == "unary"
                || parent.kind() == "argument_list"
                    && parent.parent().is_some_and(|n| n.kind() == "call")
        }) {
            replacement = format!("({replacement})");
        }
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            "Use unary plus to get an unfrozen string literal.".into(),
        );
        d.highlight_length =
            String::from_utf8_lossy(raw.split(|b| *b == b'\n').next().unwrap_or(raw))
                .chars()
                .count();
        d.correctable = !config.autocorrect_disabled()
            && std::str::from_utf8(raw).is_ok()
            && !collect_comments(node).iter().any(|c| c.start_byte() < end)
            && value.is_none_or(|v| v.kind() != "heredoc_beginning")
            && node.child_by_field_name("block").is_none();
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
    crate::cop_fixture_tests!(UnfreezeString, "cops/performance/unfreeze_string");
}
