use super::parity_helpers::*;
use crate::cop::Cop;
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

fn siblings(node: Node<'_>) -> Vec<Node<'_>> {
    node.parent()
        .filter(|parent| {
            matches!(
                parent.kind(),
                "program"
                    | "body_statement"
                    | "block_body"
                    | "then"
                    | "else"
                    | "when"
                    | "rescue"
                    | "ensure"
            )
        })
        .map(code_children)
        .unwrap_or_default()
}
pub(super) fn next_code_sibling(node: Node<'_>) -> Option<Node<'_>> {
    let siblings = siblings(node);
    let index = siblings.iter().position(|sibling| *sibling == node)?;
    siblings.get(index + 1).copied()
}
fn line_range(source: &SourceFile, line: usize) -> Option<(usize, usize)> {
    let start = source.line_start(line)?;
    let bytes = source.as_bytes();
    let mut end = source.line_start(line + 1).unwrap_or(bytes.len());
    if end > start && bytes[end - 1] == b'\n' {
        end -= 1;
    }
    Some((start, end))
}
struct CommentLines(Vec<(usize, usize, usize)>);
fn comments(source: &SourceFile, node: Node<'_>) -> std::sync::Arc<CommentLines> {
    source.cached(0, || {
        let mut root = node;
        while let Some(parent) = root.parent() {
            root = parent;
        }
        CommentLines(
            crate::cop::shared::collect_comments(root)
                .iter()
                .map(|comment| {
                    (
                        comment.start_position().row + 1,
                        comment.start_byte(),
                        comment.end_byte(),
                    )
                })
                .collect(),
        )
    })
}
pub(super) fn separate(
    cop: &dyn Cop,
    source: &SourceFile,
    node: Node<'_>,
    message: String,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    if next_code_sibling(node).is_none() {
        return;
    }
    let mut line = source.offset_to_line_col(node.end_byte()).0;
    let mut pending = vec![node];
    while let Some(child) = pending.pop() {
        if child.kind() == "heredoc_beginning" {
            if let Some(end) = crate::cop::ported::heredoc_end(source, child) {
                line = line.max(source.offset_to_line_col(end).0);
            }
        }
        pending.extend(code_children(child));
    }
    let end_line = line;
    let mut enable = None;
    let comments = comments(source, node);
    while let Some((_, start, end)) = comments
        .0
        .iter()
        .find(|(comment_line, _, _)| *comment_line == line + 1)
    {
        line += 1;
        if String::from_utf8_lossy(&source.as_bytes()[*start..*end])
            .trim_start_matches('#')
            .trim_start()
            .starts_with("rubocop:enable")
        {
            enable = Some(line);
        }
    }
    let Some((next_start, next_end)) = line_range(source, line + 1) else {
        return;
    };
    if source.as_bytes()[next_start..next_end]
        .iter()
        .all(|byte| byte.is_ascii_whitespace())
    {
        return;
    }
    let Some((start, end)) = line_range(source, enable.unwrap_or(end_line)) else {
        return;
    };
    let start = start
        + source.as_bytes()[start..end]
            .iter()
            .take_while(|byte| byte.is_ascii_whitespace())
            .count();
    let index = report(cop, source, start, end, message, diagnostics);
    if crate::cop::shared::push_replace(corrections, end, end, "\n", cop.name()) {
        diagnostics[index].corrected = true;
    }
}
pub(super) fn numbered_block(source: &SourceFile, node: Node<'_>) -> bool {
    let Some(block) = crate::cop::rspec::helpers::call_block(node) else {
        return false;
    };
    if block.child_by_field_name("parameters").is_some() {
        return false;
    }
    let mut pending = code_children(block);
    while let Some(current) = pending.pop() {
        if matches!(current.kind(), "block" | "do_block") {
            continue;
        }
        if current.kind() == "identifier"
            && name(source, current).is_some_and(|value| {
                value.len() == 2 && value[0] == b'_' && matches!(value[1], b'1'..=b'9')
            })
        {
            return true;
        }
        pending.extend(code_children(current));
    }
    false
}
