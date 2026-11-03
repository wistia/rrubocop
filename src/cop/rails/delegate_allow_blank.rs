//! Match the original two-argument delegation option without changing custom APIs.
use crate::cop::shared::ruby_arguments::arguments;
use crate::cop::shared::ruby_expression::{plain_symbol, regular_send_name, regular_send_receiver};
use crate::cop::shared::{named_kids, node_bytes, push_replace};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct DelegateAllowBlank;
struct DelegateOverrides(bool);
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
                    && node
                        .child_by_field_name("name")
                        .is_some_and(|name| node_bytes(source, name) == b"delegate")
                {
                    return DelegateOverrides(true);
                }
                if node.kind() == "alias"
                    || regular_send_name(source, node).is_some_and(|name| {
                        matches!(
                            name.as_ref(),
                            b"define_method"
                                | b"define_singleton_method"
                                | b"alias_method"
                                | b"class_eval"
                                | b"module_eval"
                                | b"eval"
                                | b"prepend"
                        )
                    })
                {
                    return DelegateOverrides(true);
                }
                pending.extend(named_kids(node));
            }
            DelegateOverrides(false)
        })
        .0
}
impl Cop for DelegateAllowBlank {
    fn name(&self) -> &'static str {
        "Rails/DelegateAllowBlank"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"delegate"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if regular_send_name(source, node).as_deref() != Some(b"delegate")
            || regular_send_receiver(source, node) != Some(None)
        {
            return;
        }
        let args = arguments(node);
        if args.len() != 2 || !args[1].hash {
            return;
        }
        let pairs = args[1].parts();
        let Some(pair) = pairs.iter().copied().find(|pair| {
            pair.kind() == "pair"
                && pair
                    .child_by_field_name("key")
                    .and_then(|key| plain_symbol(source, key))
                    .as_deref()
                    == Some("allow_blank")
                && pair
                    .child_by_field_name("value")
                    .is_some_and(|value| value.kind() == "true")
        }) else {
            return;
        };
        let key = pair.child_by_field_name("key").unwrap();
        let (line, col) = source.offset_to_line_col(pair.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            "`allow_blank` is not a valid option, use `allow_nil`.".into(),
        );
        d.highlight_length =
            super::blank::highlight_length(source, pair.start_byte(), pair.end_byte());
        d.correctable = false;
        let duplicate = pairs.iter().any(|other| {
            *other != pair
                && other
                    .child_by_field_name("key")
                    .and_then(|key| plain_symbol(source, key))
                    .is_some_and(|key| matches!(key.as_str(), "allow_nil" | "allow_blank"))
        });
        if node_bytes(source, key) == b"allow_blank"
            && !duplicate
            && !overridden(source, node)
            && !config.autocorrect_disabled()
        {
            d.correctable = true;
            d.corrected = push_replace(
                &mut corrections,
                key.start_byte(),
                key.end_byte(),
                "allow_nil",
                self.name(),
            );
        }
        diagnostics.push(d);
    }
}
