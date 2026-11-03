//! Shared implementation of Ruby's TrailingComma literal mixin.

use crate::cop::shared::{node_bytes, push_replace};
use crate::cop::style::trailing_comma_args::{
    begins_its_line, each_elem_own_line, last_item_line_start,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

fn items(node: Node<'_>) -> Vec<(usize, usize)> {
    let mut cursor = node.walk();
    let children: Vec<_> = node
        .named_children(&mut cursor)
        .filter(|child| !matches!(child.kind(), "comment" | "heredoc_body" | "heredoc_end"))
        .collect();
    let mut result = Vec::new();
    let mut index = 0;
    while index < children.len() {
        let start = children[index].start_byte();
        let mut end = children[index].end_byte();
        if node.kind() == "array" && children[index].kind() == "pair" {
            while children
                .get(index + 1)
                .is_some_and(|child| child.kind() == "pair")
            {
                index += 1;
                end = children[index].end_byte();
            }
        }
        result.push((start, end));
        index += 1;
    }
    result
}

fn comma(source: &SourceFile, node: Node<'_>, end: usize, close: usize) -> Option<usize> {
    let mut heredoc = false;
    crate::cop::shared::for_each_descendant(node, |child| {
        heredoc |= child.kind() == "heredoc_beginning"
    });
    let after = source.as_bytes().get(end..close)?;
    let offset = after.iter().position(|byte| !byte.is_ascii_whitespace())?;
    (after[offset] == b',' && (!heredoc || !after[..offset].contains(&b'\n')))
        .then_some(end + offset)
}

fn should_have_comma(
    source: &SourceFile,
    node: Node<'_>,
    items: &[(usize, usize)],
    close: usize,
    style: &str,
) -> bool {
    if node.start_position().row == node.end_position().row
        || (items.len() == 1 && !begins_its_line(source, close))
    {
        return false;
    }
    match style {
        "consistent_comma" => true,
        "comma" => each_elem_own_line(source, items, close),
        _ => false,
    }
}

pub(super) fn check_literal(
    cop: &dyn Cop,
    source: &SourceFile,
    node: Node<'_>,
    config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    mut corrections: Option<&mut Vec<Correction>>,
) {
    let (opening, article, unit) = match node.kind() {
        "array" => (b'[', "an", "array"),
        "hash" => (b'{', "a", "hash"),
        _ => return,
    };
    if node_bytes(source, node).first() != Some(&opening) {
        return;
    }
    let items = items(node);
    let Some(&(last_start, last_end)) = items.last() else {
        return;
    };
    let close = node.end_byte() - 1;
    let style = config.get_str("EnforcedStyleForMultiline", "no_comma");
    let want = should_have_comma(source, node, &items, close, style);
    let comma = comma(source, node, last_end, close);
    let (start, end, message, edit_start, edit_end, replacement) = match (comma, want) {
        (Some(comma), false) => {
            let suffix = match style {
                "comma" => ", unless each item is on its own line",
                "consistent_comma" => ", unless items are split onto multiple lines",
                _ => "",
            };
            (
                comma,
                comma + 1,
                format!("Avoid comma after the last item of {article} {unit}{suffix}."),
                comma,
                comma + 1,
                "",
            )
        }
        (None, true) => (
            last_item_line_start(source, last_start, last_end),
            last_end,
            format!("Put a comma after the last item of a multiline {unit}."),
            last_end,
            last_end,
            ",",
        ),
        _ => return,
    };
    let (line, column) = source.offset_to_line_col(start);
    let mut diagnostic = cop.diagnostic(source, line, column, message);
    diagnostic.highlight_length = String::from_utf8_lossy(&source.as_bytes()[start..end])
        .chars()
        .count();
    diagnostic.corrected = push_replace(
        &mut corrections,
        edit_start,
        edit_end,
        replacement,
        cop.name(),
    );
    diagnostics.push(diagnostic);
}
