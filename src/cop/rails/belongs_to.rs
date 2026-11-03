use crate::cop::shared::ruby_arguments::arguments;
use crate::cop::shared::ruby_expression::{plain_symbol, regular_send_name};
use crate::cop::shared::{for_each_descendant, method_node, push_replace};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct BelongsTo;
impl Cop for BelongsTo {
    fn name(&self) -> &'static str {
        "Rails/BelongsTo"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"belongs_to"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if config.get_f64("TargetRailsVersion", 8.1) < 5.0
            || !config.get_bool("__RailtiesInLockfile", true)
            || config.get_f64(
                "__RailtiesVersion",
                config.get_f64("TargetRailsVersion", 8.1),
            ) < 5.0
            || !regular_send_name(source, node).is_some_and(|n| n.as_ref() == b"belongs_to")
        {
            return;
        }
        let args = arguments(node);
        let Some(hash) = args.last().filter(|a| a.hash) else {
            return;
        };
        let parts = hash.parts();
        let Some((pair, value)) = parts
            .iter()
            .copied()
            .filter(|p| p.kind() == "pair")
            .find_map(|pair| {
                if pair
                    .child_by_field_name("key")
                    .and_then(|k| plain_symbol(source, k))
                    .as_deref()
                    != Some("required")
                {
                    return None;
                }
                let value = pair.child_by_field_name("value")?;
                matches!(value.kind(), "true" | "false").then_some((pair, value.kind()))
            })
        else {
            return;
        };
        let Some(selector) = method_node(node) else {
            return;
        };
        let (line, col) = source.offset_to_line_col(selector.start_byte());
        let (optional, message) = if value == "false" {
            (
                "true",
                "You specified `required: false`, in Rails > 5.0 the required option is deprecated and you want to use `optional: true`.",
            )
        } else {
            (
                "false",
                "You specified `required: true`, in Rails > 5.0 the required option is deprecated and you want to use `optional: false`. In most configurations, this is the default and you can omit this option altogether",
            )
        };
        let mut d = self.diagnostic(source, line, col, message.into());
        d.highlight_length =
            super::blank::highlight_length(source, selector.start_byte(), selector.end_byte());
        if let Some(s) = config.severity {
            d.severity = s;
        }
        let mut comment = false;
        for_each_descendant(pair, |n| comment |= n.kind() == "comment");
        let conflicting_optional = parts.iter().any(|p| {
            p.kind() == "pair"
                && p.start_byte() > pair.start_byte()
                && p.child_by_field_name("key")
                    .and_then(|k| plain_symbol(source, k))
                    .as_deref()
                    == Some("optional")
        });
        d.correctable = !config.autocorrect_disabled() && !comment && !conflicting_optional;
        if d.correctable {
            d.corrected = push_replace(
                &mut corrections,
                pair.start_byte(),
                pair.end_byte(),
                format!("optional: {optional}"),
                self.name(),
            );
        }
        diagnostics.push(d);
    }
}
