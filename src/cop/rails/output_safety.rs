//! Identify dynamic output bypassing Rails escaping, excluding translated values.
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, for_each_descendant, method_node, node_text,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct OutputSafety;
impl Cop for OutputSafety {
    fn name(&self) -> &'static str {
        "Rails/OutputSafety"
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"html_safe", b"raw", b"safe_concat"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let Some(method @ (b"html_safe" | b"raw" | b"safe_concat")) =
            call_method_name(source, node)
        else {
            return;
        };
        let receiver = call_receiver(node);
        if receiver.is_some_and(|n| n.kind() == "string" && !has_interpolation(n))
            || has_translation(source, node)
        {
            return;
        }
        let args = argument_nodes(node);
        let bad = match method {
            b"html_safe" => receiver.is_some() && args.is_empty(),
            b"raw" => receiver.is_none() && args.len() == 1,
            b"safe_concat" => args.len() == 1,
            _ => false,
        };
        if !bad {
            return;
        }
        let Some(method) = method_node(node) else {
            return;
        };
        let (line, col) = source.offset_to_line_col(method.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            "Tagging a string as html safe may be a security risk.".into(),
        );
        d.highlight_length = method.end_byte() - method.start_byte();
        diagnostics.push(d);
    }
}
fn has_interpolation(node: Node<'_>) -> bool {
    let mut found = false;
    for_each_descendant(node, |n| found |= n.kind() == "interpolation");
    found
}
fn has_translation(source: &SourceFile, node: Node<'_>) -> bool {
    let mut found = false;
    let attached_block = node.child_by_field_name("block");
    for_each_descendant(node, |n| {
        if attached_block.is_some_and(|block| {
            block.start_byte() <= n.start_byte() && n.end_byte() <= block.end_byte()
        }) {
            return;
        }
        if n.kind() == "identifier"
            && !n
                .parent()
                .is_some_and(|parent| parent.child_by_field_name("method") == Some(n))
            && matches!(
                node_text(source, n).as_str(),
                "t" | "translate" | "l" | "localize"
            )
            && !crate::cop::ported::local_identifier(source, n)
        {
            found = true;
        }
        if matches!(
            call_method_name(source, n),
            Some(b"t" | b"translate" | b"l" | b"localize")
        ) && call_receiver(n)
            .is_none_or(|r| matches!(node_text(source, r).as_str(), "I18n" | "::I18n"))
        {
            found = true;
        }
    });
    found
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(OutputSafety, "cops/rails/output_safety");
}
