//! Collapse an unordered single-record query into find_by.
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, method_node, node_text, push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct FindBy;
impl Cop for FindBy {
    fn name(&self) -> &'static str {
        "Rails/FindBy"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"first", b"take"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(method @ (b"first" | b"take")) = call_method_name(source, node) else {
            return;
        };
        if !argument_nodes(node).is_empty()
            || method == b"first" && config.get_bool("IgnoreWhereFirst", true)
        {
            return;
        }
        let Some(query) = call_receiver(node) else {
            return;
        };
        if call_method_name(source, query) != Some(b"where")
            || query.child_by_field_name("block").is_some()
        {
            return;
        }
        let Some(selector) = method_node(query) else {
            return;
        };
        let Some(last) = method_node(node) else {
            return;
        };
        let dot = node
            .child_by_field_name("operator")
            .map(|n| node_text(source, n))
            .unwrap_or_else(|| ".".into());
        let (line, col) = source.offset_to_line_col(selector.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            format!(
                "Use `find_by` instead of `where{dot}{}`.",
                String::from_utf8_lossy(method)
            ),
        );
        let bytes = &source.as_bytes()[selector.start_byte()..last.end_byte()];
        let end = bytes
            .iter()
            .position(|b| *b == b'\n')
            .unwrap_or(bytes.len());
        d.highlight_length = String::from_utf8_lossy(&bytes[..end]).chars().count();
        d.correctable = method == b"take"
            && !config.autocorrect_disabled()
            && !source.as_bytes()[query.end_byte()..last.end_byte()].contains(&b'#');
        if node.child_by_field_name("arguments").is_some() {
            d.correctable = false;
        }
        if d.correctable {
            d.corrected = push_replace(
                &mut corrections,
                selector.start_byte(),
                selector.end_byte(),
                "find_by",
                self.name(),
            );
            push_replace(
                &mut corrections,
                query.end_byte(),
                last.end_byte(),
                "",
                self.name(),
            );
        }
        diagnostics.push(d);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(FindBy, "cops/rails/find_by");

    #[test]
    fn does_not_produce_invalid_ruby_from_empty_outer_parentheses() {
        let before = b"User.where(id: 1).take()\n";
        let source = SourceFile::from_bytes("test.rb", before.to_vec());
        let tree = crate::parse::parse_ruby(&source).unwrap();
        let config = CopConfig::default();
        let mut diagnostics = vec![];
        let mut edits = vec![];
        crate::cop::walker::BatchedWalker::new(vec![&FindBy], vec![&config]).walk(
            &source,
            tree.root_node(),
            &mut diagnostics,
            Some(&mut edits),
        );
        assert!(edits.is_empty());
        assert_eq!(diagnostics.len(), 1);
        assert!(!diagnostics[0].correctable);
    }
}
