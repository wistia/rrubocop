//! Require string-like reflection class names, preserving original local-assignment exclusions.
use crate::cop::shared::ruby_expression::plain_symbol;
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, named_kids, node_text, push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct ReflectionClassName;
fn allowed(source: &SourceFile, node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "string" | "chained_string" | "character" | "heredoc_beginning"
    ) && crate::cop::ported::static_symbol(source, node).is_none()
        || plain_symbol(source, node).is_some()
}
fn local(source: &SourceFile, node: Node<'_>) -> bool {
    node.kind() == "identifier" && crate::cop::ported::local_identifier(source, node)
}
fn assigned_string(source: &SourceFile, pair: Node<'_>, value: Node<'_>) -> bool {
    let name = node_text(source, value);
    let mut ancestor = pair.parent();
    while let Some(node) = ancestor {
        for child in named_kids(node) {
            if child.kind() == "assignment"
                && child
                    .child_by_field_name("left")
                    .is_some_and(|n| n.kind() == "identifier" && node_text(source, n) == name)
                && child
                    .child_by_field_name("right")
                    .is_some_and(|n| allowed(source, n))
            {
                return true;
            }
        }
        ancestor = node.parent();
    }
    false
}
impl Cop for ReflectionClassName {
    fn name(&self) -> &'static str {
        "Rails/ReflectionClassName"
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
        &[b"has_many", b"has_one", b"belongs_to"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if !super::unknown_env::regular(node, source)
            || call_receiver(node).is_some()
            || crate::cop::shared::is_setter_call(node)
            || !matches!(
                call_method_name(source, node),
                Some(b"has_many" | b"has_one" | b"belongs_to")
            )
        {
            return;
        }
        let args = argument_nodes(node);
        let Some(last) = args.last().copied() else {
            return;
        };
        let pairs = if last.kind() == "hash" {
            if !(2..=3).contains(&args.len()) {
                return;
            }
            named_kids(last)
        } else {
            let Some(start) = args
                .iter()
                .position(|n| matches!(n.kind(), "pair" | "hash_splat_argument"))
            else {
                return;
            };
            if !(1..=2).contains(&start) {
                return;
            }
            args[start..].to_vec()
        };
        for pair in pairs {
            if pair.kind() != "pair"
                || !pair
                    .child_by_field_name("key")
                    .is_some_and(|n| plain_symbol(source, n).as_deref() == Some("class_name"))
            {
                continue;
            }
            let shorthand = pair.child_by_field_name("value").is_none();
            let value = pair
                .child_by_field_name("value")
                .or_else(|| pair.child_by_field_name("key"))
                .unwrap();
            if !shorthand && allowed(source, value) {
                continue;
            }
            let is_local = if shorthand {
                crate::cop::ported::local_identifier(source, value)
            } else {
                local(source, value)
            };
            if shorthand && !is_local {
                continue;
            }
            let bare = crate::cop::ported::bare_call_identifier(source, value);
            let send = super::unknown_env::regular(value, source) || bare;
            if send
                && (call_receiver(value).is_none()
                    || call_method_name(source, value) == Some(b"to_s")
                        && !call_receiver(value)
                            .is_some_and(|n| matches!(n.kind(), "constant" | "scope_resolution")))
            {
                continue;
            }
            if is_local && assigned_string(source, pair, value) {
                continue;
            }
            let replacement = if value.kind() == "constant" {
                Some(value)
            } else if super::unknown_env::regular(value, source)
                && matches!(call_method_name(source, value), Some(b"name" | b"to_s"))
                && argument_nodes(value).is_empty()
                && value.child_by_field_name("block").is_none()
            {
                call_receiver(value).filter(|n| n.kind() == "constant")
            } else {
                None
            };
            let (line, col) = source.offset_to_line_col(pair.start_byte());
            let mut d = self.diagnostic(
                source,
                line,
                col,
                "Use a string value for `class_name`.".into(),
            );
            d.highlight_length =
                super::blank::highlight_length(source, pair.start_byte(), pair.end_byte());
            d.correctable = !config.autocorrect_disabled() && replacement.is_some();
            if let Some(replacement) = replacement.filter(|_| d.correctable) {
                d.corrected = push_replace(
                    &mut corrections,
                    value.start_byte(),
                    value.end_byte(),
                    crate::cop::shared::ruby_literal::string_dump(&node_text(source, replacement)),
                    self.name(),
                );
            }
            diagnostics.push(d);
            break;
        }
    }
}
