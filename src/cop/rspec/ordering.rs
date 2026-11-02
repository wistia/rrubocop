use super::helpers::{bare_rspec_call, call_block, is_example, is_group};
use super::parity_helpers::*;
use super::separation::numbered_block;
use crate::cop::shared::{call_receiver, push_replace};
use crate::correction::Correction;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub(super) fn helper(source: &SourceFile, node: Node<'_>, methods: &[&[u8]]) -> bool {
    if call_receiver(node).is_some()
        || safe_send(source, node)
        || !name(source, node).is_some_and(|name| methods.contains(&name))
    {
        return false;
    }
    if call_block(node).is_some() {
        return !numbered_block(source, node);
    }
    let args = call_args(node);
    args.len() == 2
        && args
            .last()
            .is_some_and(|arg| arg.kind() == "block_argument")
}
pub(super) fn hook(source: &SourceFile, node: Node<'_>) -> bool {
    call_block(node).is_some()
        && call_receiver(node).is_none()
        && !safe_send(source, node)
        && name(source, node).is_some_and(|method| hooks().contains(&method))
}
pub(super) fn includes(source: &SourceFile, node: Node<'_>) -> bool {
    !numbered_block(source, node)
        && call_receiver(node).is_none()
        && !safe_send(source, node)
        && matches!(
            name(source, node),
            Some(
                b"include_examples"
                    | b"include_context"
                    | b"it_behaves_like"
                    | b"it_should_behave_like"
            )
        )
}
pub(super) fn example(source: &SourceFile, node: Node<'_>) -> bool {
    call_block(node).is_some()
        && !numbered_block(source, node)
        && call_receiver(node).is_none()
        && name(source, node).is_some_and(is_example)
}
pub(super) fn group(source: &SourceFile, node: Node<'_>) -> bool {
    call_block(node).is_some() && bare_rspec_call(source, node).is_some_and(is_group)
}
pub(super) fn body<'a>(node: Node<'a>) -> Vec<Node<'a>> {
    let Some(body) = call_block(node).and_then(|block| block.child_by_field_name("body")) else {
        return Vec::new();
    };
    code_children(body)
        .into_iter()
        .filter(|node| node.kind() != "heredoc_body")
        .collect()
}
fn line_start(source: &SourceFile, offset: usize) -> usize {
    source
        .line_start(source.offset_to_line_col(offset).0)
        .unwrap_or(0)
}
fn line_end(source: &SourceFile, offset: usize) -> usize {
    let line = source.offset_to_line_col(offset).0;
    let end = source
        .line_start(line + 1)
        .unwrap_or(source.as_bytes().len());
    end - usize::from(end > 0 && source.as_bytes()[end - 1] == b'\n')
}
fn final_end(source: &SourceFile, node: Node<'_>) -> usize {
    let mut end = node.end_byte();
    let mut pending = vec![node];
    while let Some(child) = pending.pop() {
        if child.kind() == "heredoc_beginning" {
            end = end.max(
                crate::cop::ported::heredoc_end(source, child)
                    .map(|end| line_end(source, end))
                    .unwrap_or(end),
            );
        }
        pending.extend(code_children(child));
    }
    end
}
fn with_comments(source: &SourceFile, node: Node<'_>) -> (usize, usize) {
    let mut start = line_start(source, node.start_byte());
    let mut probe = start;
    while probe > 0 {
        let previous = line_start(source, probe - 1);
        let line = &source.as_bytes()[previous..probe];
        let content = line
            .iter()
            .position(|byte| !byte.is_ascii_whitespace())
            .map(|index| &line[index..]);
        if content.is_some_and(|line| line.starts_with(b"#")) {
            start = previous;
        } else if content.is_some() {
            break;
        }
        probe = previous;
    }
    (start, line_end(source, final_end(source, node)))
}
pub(super) fn move_node(
    source: &SourceFile,
    node: Node<'_>,
    target: Node<'_>,
    after: bool,
    corrections: &mut Option<&mut Vec<Correction>>,
    cop: &'static str,
) -> bool {
    if corrections.is_none() {
        return false;
    }
    let (start, end) = with_comments(source, node);
    let mut root = node;
    while let Some(parent) = root.parent() {
        root = parent;
    }
    let mut pending = vec![root];
    while let Some(child) = pending.pop() {
        if child.kind() == "comment"
            && child.start_byte() >= start
            && child.end_byte() <= end
            && source.as_bytes()[child.start_byte()..child.end_byte()]
                .windows(8)
                .any(|bytes| bytes == b"rubocop:")
        {
            return false;
        }
        let mut cursor = child.walk();
        pending.extend(child.named_children(&mut cursor));
    }

    if !source.as_bytes()[line_start(source, node.start_byte())..node.start_byte()]
        .iter()
        .all(u8::is_ascii_whitespace)
    {
        return false;
    }
    let suffix = &source.as_bytes()[final_end(source, node)..end];
    if !suffix.iter().all(u8::is_ascii_whitespace)
        && !suffix
            .iter()
            .position(|byte| !byte.is_ascii_whitespace())
            .is_some_and(|index| suffix[index] == b'#')
    {
        return false;
    }
    if !source.as_bytes()[line_start(source, target.start_byte())..target.start_byte()]
        .iter()
        .all(u8::is_ascii_whitespace)
    {
        return false;
    }
    let position = if after {
        line_end(source, final_end(source, target))
    } else {
        line_start(source, target.start_byte())
    };
    if (start..=end).contains(&position) {
        return false;
    }
    let text = String::from_utf8_lossy(&source.as_bytes()[start..end]);
    let replacement = if after {
        format!("\n{text}")
    } else {
        format!("{text}\n")
    };
    let before = corrections.as_ref().map_or(0, |edits| edits.len());
    if push_replace(corrections, position, position, replacement, cop) {
        let end = end + usize::from(source.as_bytes().get(end) == Some(&b'\n'));
        push_replace(corrections, start, end, "", cop);
        if let Some(edits) = corrections.as_deref_mut() {
            for edit in &mut edits[before..] {
                edit.owner = Some(node.start_byte());
            }
        }
        true
    } else {
        false
    }
}
pub(super) fn stable_arguments(source: &SourceFile, node: Node<'_>) -> bool {
    let args = call_args(node);
    if matches!(name(source, node), Some(b"let" | b"let!")) && args.len() != 1
        || matches!(name(source, node), Some(b"subject" | b"subject!")) && args.len() > 1
    {
        return false;
    }
    args.iter().all(|argument| {
        crate::cop::ported::static_symbol(source, *argument).is_some()
            || argument.kind() == "string"
                && crate::cop::ported::static_literal(source, *argument).is_some()
    })
}
pub(super) fn definition_name(source: &SourceFile, node: Node<'_>) -> Option<String> {
    let args = call_args(node);
    if args.is_empty() && matches!(name(source, node), Some(b"subject" | b"subject!")) {
        Some("subject".into())
    } else {
        args.first()
            .and_then(|arg| crate::cop::ported::static_literal(source, *arg))
    }
}

#[derive(Clone, Copy)]
pub(super) enum Style {
    HooksBeforeExamples,
    LetBeforeExamples,
    ScatteredLet,
}
fn marker(source: &SourceFile, node: Node<'_>, hooks: bool) -> bool {
    let has_block = call_block(node).is_some();
    let named_group = group(source, node)
        && bare_rspec_call(source, node).is_some_and(is_example_group)
        && (hooks || call_receiver(node).is_none() && !numbered_block(source, node));
    let named_example = call_receiver(node).is_none()
        && name(source, node).is_some_and(is_example)
        && has_block
        && (hooks || !numbered_block(source, node));
    named_group
        || named_example
        || !has_block && includes(source, node) && name(source, node) != Some(b"include_context")
}
fn eager(source: &SourceFile, node: Node<'_>) -> bool {
    helper(source, node, &[b"let!", b"subject!"])
}
pub(super) fn check(
    cop: &dyn crate::cop::Cop,
    style: Style,
    source: &SourceFile,
    node: Node<'_>,
    diagnostics: &mut Vec<crate::diagnostic::Diagnostic>,
    mut corrections: Option<&mut Vec<Correction>>,
) {
    if numbered_block(source, node)
        || !bare_rspec_call(source, node).is_some_and(is_example_group)
        || call_block(node).is_none()
    {
        return;
    }
    let children = body(node);
    if children.len() < 2 {
        return;
    }
    let scattered = matches!(style, Style::ScatteredLet);
    let hooks_first = matches!(style, Style::HooksBeforeExamples);
    let Some(first) = children.iter().position(|child| {
        if scattered {
            helper(source, *child, &[b"let", b"let!"])
        } else {
            marker(source, *child, hooks_first)
        }
    }) else {
        return;
    };
    let no_include_examples = !children.iter().any(|child| {
        !numbered_block(source, *child)
            && includes(source, *child)
            && name(source, *child) == Some(b"include_examples")
    });
    let mut candidates: Vec<_> = children.iter().copied().enumerate().skip(first).collect();
    if !hooks_first {
        candidates.reverse();
    }
    for (index, child) in candidates {
        if scattered {
            if !helper(source, child, &[b"let", b"let!"]) {
                continue;
            }
            let expected = first
                + children[first..index]
                    .iter()
                    .filter(|child| helper(source, **child, &[b"let", b"let!"]))
                    .count();
            if index == expected {
                continue;
            }
        } else if index == first
            || !(if hooks_first {
                hook(source, child)
            } else {
                helper(source, child, &[b"let", b"let!"])
            })
        {
            continue;
        }
        let message = if scattered {
            "Group all let/let! blocks in the example group together.".into()
        } else if hooks_first {
            format!(
                "Move `{}` above the examples in the group.",
                String::from_utf8_lossy(name(source, child).unwrap())
            )
        } else {
            "Move `let` before the examples in the group.".into()
        };
        let diagnostic = report(
            cop,
            source,
            child.start_byte(),
            child.end_byte(),
            message,
            diagnostics,
        );
        if !hooks_first && !scattered && !no_include_examples {
            diagnostics[diagnostic].correctable = false;
            continue;
        }
        let crossed = &children[first..index];
        let moving_eager = eager(source, child);
        let moving_before = hooks_first
            && matches!(
                name(source, child),
                Some(b"before" | b"prepend_before" | b"append_before")
            );
        if !stable_arguments(source, child)
            || crossed.iter().skip(usize::from(scattered)).any(|sibling| {
                includes(source, *sibling)
                    || (moving_eager && (hook(source, *sibling) || eager(source, *sibling)))
                    || (moving_before && eager(source, *sibling))
                    || (!hooks_first
                        && helper(
                            source,
                            *sibling,
                            &[b"let", b"let!", b"subject", b"subject!"],
                        )
                        && definition_name(source, *sibling) == definition_name(source, child))
            })
        {
            continue;
        }
        if move_node(
            source,
            child,
            children[first],
            scattered,
            &mut corrections,
            cop.name(),
        ) {
            diagnostics[diagnostic].corrected = true;
        }
    }
}
