//! Reuse the default URI parser when no constructor options are supplied.
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, node_text, push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct UriDefaultParser;
impl Cop for UriDefaultParser {
    fn name(&self) -> &'static str {
        "Performance/UriDefaultParser"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"new"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if call_method_name(source, node) != Some(b"new") || !argument_nodes(node).is_empty() {
            return;
        }
        let Some(receiver) = call_receiver(node) else {
            return;
        };
        let value = node_text(source, receiver);
        if !matches!(value.as_str(), "URI::Parser" | "::URI::Parser") {
            return;
        }
        if node
            .child_by_field_name("operator")
            .is_some_and(|n| node_text(source, n) == "&.")
        {
            return;
        }
        let prefix = if value.starts_with("::") { "::" } else { "" };
        let replacement = format!("{prefix}URI::DEFAULT_PARSER");
        let end = node
            .child_by_field_name("block")
            .map_or(node.end_byte(), |block| {
                source.as_bytes()[node.start_byte()..block.start_byte()]
                    .iter()
                    .rposition(|b| !b.is_ascii_whitespace())
                    .map_or(block.start_byte(), |last| node.start_byte() + last + 1)
            });
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            format!("Use `{replacement}` instead of `{prefix}URI::Parser.new`."),
        );
        d.highlight_length = end - node.start_byte();
        d.corrected = push_replace(
            &mut corrections,
            node.start_byte(),
            end,
            replacement,
            self.name(),
        );
        d.correctable &= !config.autocorrect_disabled();
        diagnostics.push(d);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(UriDefaultParser, "cops/performance/uri_default_parser");
}
