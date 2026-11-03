use crate::cop::shared::ruby_arguments::arguments;
use crate::cop::shared::ruby_expression::{plain_symbol, regular_send_name, regular_send_receiver};
use crate::cop::shared::{for_each_descendant, node_text, push_replace, send_end};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct ReadWriteAttribute;
fn shadowed(source: &SourceFile, node: Node<'_>, first: Node<'_>, write: bool) -> bool {
    let Some(value) = plain_symbol(source, first)
        .or_else(|| crate::cop::ported::original_string_value(source, first))
    else {
        return false;
    };
    let wanted = if write { format!("{value}=") } else { value };
    let mut parent = node.parent();
    while let Some(ancestor) = parent {
        if ancestor.kind() == "method" {
            return ancestor
                .child_by_field_name("name")
                .is_some_and(|n| node_text(source, n) == wanted);
        }
        parent = ancestor.parent();
    }
    false
}
fn unsafe_write_context(source: &SourceFile, node: Node<'_>) -> bool {
    if node.child_by_field_name("block").is_some() {
        return true;
    }
    node.parent().is_some_and(|p| {
        if p.kind() == "binary" {
            let op = p
                .child_by_field_name("operator")
                .map(|n| node_text(source, n))
                .unwrap_or_default();
            return !matches!(op.as_str(), "and" | "or")
                && !(matches!(op.as_str(), "&&" | "||")
                    && p.child_by_field_name("right") == Some(node));
        }
        matches!(p.kind(), "unary" | "range" | "conditional")
            || p.kind() == "call" && p.child_by_field_name("receiver") == Some(node)
    })
}
impl Cop for ReadWriteAttribute {
    fn name(&self) -> &'static str {
        "Rails/ReadWriteAttribute"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn default_include(&self) -> &'static [&'static str] {
        &["app/models/**/*.rb"]
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"read_attribute", b"write_attribute"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if regular_send_receiver(source, node) != Some(None) {
            return;
        }
        let Some(method) = regular_send_name(source, node) else {
            return;
        };
        let write = match method.as_ref() {
            b"read_attribute" => false,
            b"write_attribute" => true,
            _ => return,
        };
        let args = arguments(node);
        if args.len() != if write { 2 } else { 1 } {
            return;
        }
        if args[0]
            .node()
            .is_some_and(|first| shadowed(source, node, first, write))
        {
            return;
        }
        let first = String::from_utf8_lossy(&source.as_bytes()[args[0].start()..args[0].end()]);
        let replacement = if write {
            format!(
                "self[{first}] = {}",
                String::from_utf8_lossy(&source.as_bytes()[args[1].start()..args[1].end()])
            )
        } else {
            format!("self[{first}]")
        };
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let multiline = source.offset_to_line_col(send_end(node)).0 != line;
        let message = if multiline {
            if write {
                "self[:attr] = val"
            } else {
                "self[:attr]"
            }
        } else {
            &replacement
        };
        let mut d = self.diagnostic(source, line, col, format!("Prefer `{message}`."));
        d.highlight_length =
            super::blank::highlight_length(source, node.start_byte(), send_end(node));
        if let Some(s) = config.severity {
            d.severity = s;
        }
        let mut comment = false;
        for_each_descendant(node, |n| comment |= n.kind() == "comment");
        d.correctable = !config.autocorrect_disabled()
            && !comment
            && !(write && (unsafe_write_context(source, node) || args[1].hash && !args[1].braces));
        if d.correctable {
            d.corrected = push_replace(
                &mut corrections,
                node.start_byte(),
                send_end(node),
                replacement,
                self.name(),
            );
        }
        diagnostics.push(d);
    }
}
