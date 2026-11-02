//! Search a hash directly rather than allocating its key or value array.
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, collect_comments, node_text, push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct InefficientHashSearch;
impl Cop for InefficientHashSearch {
    fn name(&self) -> &'static str {
        "Performance/InefficientHashSearch"
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
        &[b"include?"]
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
        if call_method_name(source, node) != Some(b"include?") {
            return;
        }
        let args = argument_nodes(node);
        if args.len() != 1 {
            return;
        }
        let Some(array) = call_receiver(node) else {
            return;
        };
        let Some(method @ (b"keys" | b"values")) = call_method_name(source, array) else {
            return;
        };
        if !argument_nodes(array).is_empty() || array.child_by_field_name("block").is_some() {
            return;
        }
        let Some(hash) = call_receiver(array) else {
            return;
        };
        let preferred = if method == b"keys" { "key?" } else { "value?" };
        let dot = array
            .child_by_field_name("operator")
            .map(|n| node_text(source, n))
            .unwrap_or_else(|| ".".into());
        let header_end = super::send_range::end(node);
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            format!(
                "Use `#{preferred}` instead of `#{}.include?`.",
                String::from_utf8_lossy(method)
            ),
        );
        let raw = &source.as_bytes()[node.start_byte()..header_end];
        let end = raw.iter().position(|b| *b == b'\n').unwrap_or(raw.len());
        d.highlight_length = String::from_utf8_lossy(&raw[..end]).chars().count();
        d.correctable = !config.autocorrect_disabled()
            && std::str::from_utf8(raw).is_ok()
            && !collect_comments(node)
                .iter()
                .any(|n| n.start_byte() < header_end);
        if d.correctable {
            let replacement = format!(
                "{}{dot}{preferred}({})",
                node_text(source, hash),
                node_text(source, args[0])
            );
            d.corrected = push_replace(
                &mut corrections,
                node.start_byte(),
                header_end,
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
    crate::cop_fixture_tests!(
        InefficientHashSearch,
        "cops/performance/inefficient_hash_search"
    );
}
