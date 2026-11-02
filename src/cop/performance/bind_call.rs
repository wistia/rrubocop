//! Combine unbound-method binding and invocation.
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, collect_comments, method_node, node_text,
    push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct BindCall;
impl Cop for BindCall {
    fn name(&self) -> &'static str {
        "Performance/BindCall"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if config.get_f64("TargetRubyVersion", 3.4) < 2.7
            || !matches!(call_method_name(source, node), Some(b"call") | None)
        {
            return;
        }
        let Some(bind) = call_receiver(node) else {
            return;
        };
        if call_method_name(source, bind) != Some(b"bind")
            || bind.child_by_field_name("block").is_some()
            || [node, bind].iter().any(|n| {
                n.child_by_field_name("operator")
                    .is_some_and(|op| node_text(source, op) == "&.")
            })
        {
            return;
        }
        let args = argument_nodes(bind);
        let [arg] = args.as_slice() else {
            return;
        };
        let Some(selector) = method_node(bind) else {
            return;
        };
        let bind_arg = node_text(source, *arg);
        let call_args = argument_nodes(node)
            .into_iter()
            .map(|n| node_text(source, n))
            .collect::<Vec<_>>()
            .join(", ");
        let comma = if call_args.is_empty() { "" } else { ", " };
        let end = super::send_range::end(node);
        let (line, col) = source.offset_to_line_col(selector.start_byte());
        let mut d=self.diagnostic(source,line,col,format!("Use `bind_call({bind_arg}{comma}{call_args})` instead of `bind({bind_arg}).call({call_args})`."));
        let raw = &source.as_bytes()[selector.start_byte()..end];
        d.highlight_length =
            String::from_utf8_lossy(raw.split(|b| *b == b'\n').next().unwrap_or(raw))
                .chars()
                .count();
        d.correctable = !config.autocorrect_disabled()
            && std::str::from_utf8(raw).is_ok()
            && !collect_comments(node)
                .iter()
                .any(|c| c.start_byte() >= selector.start_byte() && c.start_byte() < end);
        if d.correctable {
            d.corrected = push_replace(
                &mut corrections,
                selector.start_byte(),
                end,
                format!("bind_call({bind_arg}{comma}{call_args})"),
                self.name(),
            );
        }
        diagnostics.push(d);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(BindCall, "cops/performance/bind_call");
}
