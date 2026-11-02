//! Iterate in reverse without materializing an unused reversed array.
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, collect_comments, method_node, named_kids,
    node_bytes, push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct ReverseEach;
impl Cop for ReverseEach {
    fn name(&self) -> &'static str {
        "Performance/ReverseEach"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"each"]
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
        if call_method_name(source, node) != Some(b"each")
            || !argument_nodes(node).is_empty()
            || return_value_used(source, node)
        {
            return;
        }
        let Some(reverse) = call_receiver(node) else {
            return;
        };
        if call_method_name(source, reverse) != Some(b"reverse")
            || !argument_nodes(reverse).is_empty()
            || reverse.child_by_field_name("block").is_some()
        {
            return;
        }
        let Some(first) = method_node(reverse) else {
            return;
        };
        let Some(last) = method_node(node) else {
            return;
        };
        let (line, col) = source.offset_to_line_col(first.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            "Use `reverse_each` instead of `reverse.each`.".into(),
        );
        let range = &source.as_bytes()[first.start_byte()..last.end_byte()];
        let end = range
            .iter()
            .position(|b| *b == b'\n')
            .unwrap_or(range.len());
        d.highlight_length = String::from_utf8_lossy(&range[..end]).chars().count();
        d.correctable = !config.autocorrect_disabled()
            && !implicit_method_return(node)
            && !collect_comments(node)
                .iter()
                .any(|n| n.start_byte() >= first.start_byte() && n.end_byte() <= last.end_byte());
        if d.correctable {
            d.corrected = push_replace(
                &mut corrections,
                first.start_byte(),
                last.end_byte(),
                "reverse_each",
                self.name(),
            );
        }
        diagnostics.push(d);
    }
}
fn return_value_used(source: &SourceFile, node: Node<'_>) -> bool {
    let mut parent = node.parent();
    while let Some(ancestor) = parent {
        if matches!(
            ancestor.kind(),
            "assignment" | "operator_assignment" | "return"
        ) {
            return true;
        }
        if ancestor.kind() == "call"
            && ancestor
                .child_by_field_name("operator")
                .is_none_or(|op| node_bytes(source, op) != b"&.")
        {
            let block_contains = ancestor.child_by_field_name("block").is_some_and(|b| {
                b.start_byte() <= node.start_byte() && node.end_byte() <= b.end_byte()
            });
            if !block_contains {
                return true;
            }
        }
        parent = ancestor.parent();
    }
    false
}
fn implicit_method_return(node: Node<'_>) -> bool {
    let mut parent = node.parent();
    while let Some(ancestor) = parent {
        if matches!(ancestor.kind(), "method" | "singleton_method") {
            let Some(body) = ancestor.child_by_field_name("body") else {
                return false;
            };
            return named_kids(body)
                .into_iter()
                .filter(|n| n.kind() != "comment")
                .last()
                .is_some_and(|last| {
                    last.start_byte() <= node.start_byte() && node.end_byte() <= last.end_byte()
                });
        }
        parent = ancestor.parent();
    }
    false
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ReverseEach, "cops/performance/reverse_each");

    #[test]
    fn preserves_implicit_return_values() {
        let before = b"def order; [1, 2, 3].reverse.each {}; end\n";
        let source = SourceFile::from_bytes("test.rb", before.to_vec());
        let tree = crate::parse::parse_ruby(&source).unwrap();
        let config = CopConfig::default();
        let mut diagnostics = vec![];
        let mut edits = vec![];
        crate::cop::walker::BatchedWalker::new(vec![&ReverseEach], vec![&config]).walk(
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
