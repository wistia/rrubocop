//! Preserve implicit request getter dispatch while selecting the configured accessor alias.
use crate::cop::shared::ruby_expression::{regular_send_name, regular_send_receiver};
use crate::cop::shared::{
    argument_nodes, collect_comments, named_kids, node_bytes, node_text, push_replace, send_end,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct RequestReferer;
struct RequestOverrides(bool);
fn overridden(source: &SourceFile, node: Node<'_>) -> bool {
    source
        .cached(0, || {
            let mut root = node;
            while let Some(parent) = root.parent() {
                root = parent;
            }
            let mut pending = vec![root];
            while let Some(node) = pending.pop() {
                if matches!(node.kind(), "method" | "singleton_method")
                    && node.child_by_field_name("name").is_some_and(|name| {
                        matches!(node_bytes(source, name), b"referer" | b"referrer")
                    })
                {
                    return RequestOverrides(true);
                }
                if node.kind() == "alias"
                    || regular_send_name(source, node).is_some_and(|name| {
                        matches!(
                            name.as_ref(),
                            b"alias_method"
                                | b"define_method"
                                | b"define_singleton_method"
                                | b"class_eval"
                                | b"module_eval"
                                | b"eval"
                                | b"prepend"
                        )
                    })
                {
                    return RequestOverrides(true);
                }
                pending.extend(named_kids(node));
            }
            RequestOverrides(false)
        })
        .0
}
impl Cop for RequestReferer {
    fn name(&self) -> &'static str {
        "Rails/RequestReferer"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"referer", b"referrer"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let prefer = config.get_str("EnforcedStyle", "referer");
        let wrong = if prefer == "referer" {
            "referrer"
        } else {
            "referer"
        };
        if regular_send_name(source, node).as_deref() != Some(wrong.as_bytes())
            || !argument_nodes(node).is_empty()
        {
            return;
        }
        let Some(Some(receiver)) = regular_send_receiver(source, node) else {
            return;
        };
        if regular_send_name(source, receiver).as_deref() != Some(b"request")
            || regular_send_receiver(source, receiver) != Some(None)
            || !argument_nodes(receiver).is_empty()
            || receiver.child_by_field_name("block").is_some()
        {
            return;
        }
        let end = send_end(node);
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            format!("Use `request.{prefer}` instead of `request.{wrong}`."),
        );
        d.highlight_length = super::blank::highlight_length(source, node.start_byte(), end);
        d.correctable = !config.autocorrect_disabled() && !overridden(source, node);
        let getter = if receiver.kind() == "identifier" {
            "request".into()
        } else {
            node_text(source, receiver)
        };
        let replacement = format!("{getter}.{prefer}");
        if collect_comments(node)
            .iter()
            .any(|comment| comment.start_byte() < end)
        {
            d.correctable = false;
        }
        let removes_lines = source.as_bytes()[node.start_byte()..end]
            .iter()
            .filter(|b| **b == b'\n')
            .count()
            != replacement.bytes().filter(|b| *b == b'\n').count();
        if removes_lines {
            struct LineSensitive(bool);
            let sensitive = source.cached(0, || {
                let mut root = node;
                while let Some(parent) = root.parent() {
                    root = parent;
                }
                let mut pending = vec![root];
                while let Some(node) = pending.pop() {
                    if crate::cop::shared::is_line_literal(source, node) {
                        return LineSensitive(true);
                    }
                    pending.extend(named_kids(node));
                }
                LineSensitive(false)
            });
            d.correctable &= !sensitive.0;
        }
        if d.correctable {
            let mut proposed = source.as_bytes().to_vec();
            proposed.splice(node.start_byte()..end, replacement.bytes());
            let proposed = SourceFile::from_bytes("request.rb", proposed);
            d.correctable =
                crate::parse::parse_ruby(&proposed).is_ok_and(|tree| !tree.root_node().has_error());
        }
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
