//! Collapse a filtering block followed by a count into one traversal.

use tree_sitter::Node;

use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, method_node, node_text, push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct Count;

impl Cop for Count {
    fn name(&self) -> &'static str {
        "Performance/Count"
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
        &[b"count", b"length", b"size"]
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
        let Some(counter @ (b"count" | b"length" | b"size")) = call_method_name(source, node)
        else {
            return;
        };
        if !argument_nodes(node).is_empty() || node.child_by_field_name("block").is_some() {
            return;
        }
        let Some(filter) = call_receiver(node) else {
            return;
        };
        let Some(selector @ (b"select" | b"filter" | b"find_all" | b"reject")) =
            call_method_name(source, filter)
        else {
            return;
        };
        let block = filter.child_by_field_name("block");
        if config.get_f64("TargetRubyVersion", 3.4) >= 2.7
            && block.is_some_and(|block| {
                crate::cop::shared::ruby_expression::numbered_bindings(source, block).is_some()
            })
        {
            return;
        }
        let args = argument_nodes(filter);
        let pass = args
            .first()
            .filter(|n| args.len() == 1 && n.kind() == "block_argument")
            .copied();
        if block.is_none() && pass.is_none() {
            return;
        }
        if block.is_some() && !args.is_empty() {
            return;
        }
        let Some(method) = method_node(filter) else {
            return;
        };
        let (line, col) = source.offset_to_line_col(method.start_byte());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            col,
            format!(
                "Use `count` instead of `{}...{}`.",
                String::from_utf8_lossy(selector),
                String::from_utf8_lossy(counter)
            ),
        );
        let highlighted = &source.as_bytes()[method.start_byte()..node.end_byte()];
        let line_end = highlighted
            .iter()
            .position(|byte| *byte == b'\n')
            .unwrap_or(highlighted.len());
        diagnostic.highlight_length = String::from_utf8_lossy(&highlighted[..line_end])
            .chars()
            .count();
        let edits = edits(source, node, filter, selector == b"reject", block, pass);
        diagnostic.correctable = edits.is_some();
        if let Some(edits) = edits {
            for (start, end, replacement) in edits {
                diagnostic.corrected |=
                    push_replace(&mut corrections, start, end, replacement, self.name());
            }
        }
        diagnostic.correctable &= !config.autocorrect_disabled();
        diagnostics.push(diagnostic);
    }
}

fn edits(
    source: &SourceFile,
    node: Node<'_>,
    filter: Node<'_>,
    reject: bool,
    block: Option<Node<'_>>,
    pass: Option<Node<'_>>,
) -> Option<Vec<(usize, usize, String)>> {
    let method = method_node(filter)?;
    if let Some(block) = block {
        let mut changes_control_value = false;
        crate::cop::shared::for_each_descendant(block, |n| {
            changes_control_value |=
                n.kind() == "break" || reject && matches!(n.kind(), "next" | "redo" | "retry");
        });
        if changes_control_value {
            return None;
        }
    }
    if source.as_bytes()[filter.end_byte()..node.end_byte()].contains(&b'#') {
        return None;
    }
    let mut edits = vec![
        (method.start_byte(), method.end_byte(), "count".to_owned()),
        (filter.end_byte(), node.end_byte(), String::new()),
    ];
    if !reject {
        return Some(edits);
    }
    if let Some(block) = block {
        let body = block.child_by_field_name("body")?;
        let mut cursor = body.walk();
        let last = body
            .named_children(&mut cursor)
            .filter(|n| n.kind() != "comment")
            .last()?;
        if matches!(last.kind(), "next" | "return" | "break" | "redo" | "retry") {
            return None;
        }
        edits.push((last.start_byte(), last.start_byte(), "!(".into()));
        edits.push((last.end_byte(), last.end_byte(), ")".into()));
    } else {
        let value = pass?.named_child(0)?;
        if value.kind() != "simple_symbol" {
            return None;
        }
        let args = filter.child_by_field_name("arguments")?;
        if !crate::cop::shared::collect_comments(args).is_empty() {
            return None;
        }
        edits.push((
            args.start_byte(),
            args.end_byte(),
            format!(
                " {{ |element| !element.{} }}",
                node_text(source, value).trim_start_matches(':')
            ),
        ));
    }
    Some(edits)
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Count, "cops/performance/count");

    fn corrected(bytes: &[u8]) -> (Vec<u8>, Vec<Diagnostic>) {
        let source = SourceFile::from_bytes("test.rb", bytes.to_vec());
        let tree = crate::parse::parse_ruby(&source).unwrap();
        let config = CopConfig::default();
        let mut diagnostics = vec![];
        let mut edits = vec![];
        crate::cop::walker::BatchedWalker::new(vec![&Count], vec![&config]).walk(
            &source,
            tree.root_node(),
            &mut diagnostics,
            Some(&mut edits),
        );
        (
            crate::correction::CorrectionSet::from_vec(edits).apply(bytes),
            diagnostics,
        )
    }

    #[test]
    fn retains_block_comments_and_raw_bytes() {
        let before = b"items.reject { |item| # keep this\n item == \"\xff\" }.count\n";
        let expected = b"items.count { |item| # keep this\n !(item == \"\xff\") }\n";
        assert_eq!(corrected(before).0, expected);
    }

    #[test]
    fn does_not_discard_proc_arguments() {
        let before = b"items.reject(&predicate).count\n";
        let (after, diagnostics) = corrected(before);
        assert_eq!(after, before);
        assert_eq!(diagnostics.len(), 1);
        assert!(!diagnostics[0].correctable);
    }

    #[test]
    fn preserves_block_control_flow_values() {
        for before in [
            b"items.select { |item| break [7] }.count\n".as_slice(),
            b"items.reject { |item| if item == 1; next false; end; true }.count\n",
        ] {
            let (after, diagnostics) = corrected(before);
            assert_eq!(after, before);
            assert_eq!(diagnostics.len(), 1);
            assert!(!diagnostics[0].correctable);
        }
    }

    #[test]
    fn does_not_delete_a_comment_between_calls() {
        let before = b"items.select { |item| item.active? } # keep this\n .count\n";
        let (after, diagnostics) = corrected(before);
        assert_eq!(after, before);
        assert_eq!(diagnostics.len(), 1);
        assert!(!diagnostics[0].correctable);
    }
}
