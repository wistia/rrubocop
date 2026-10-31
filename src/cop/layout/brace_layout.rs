//! Shared brace style check for Layout/Multiline*BraceLayout cops.

use tree_sitter::Node;

use crate::cop::Cop;
use crate::cop::shared;
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

fn style_ok(style: &str, new_line_open: bool, new_line_close: bool) -> bool {
    match style {
        "new_line" => new_line_open && new_line_close,
        "same_line" => !new_line_open && !new_line_close,
        _ => new_line_open == new_line_close,
    }
}

fn want_new_line_close(style: &str, new_line_open: bool) -> bool {
    match style {
        "new_line" => true,
        "same_line" => false,
        _ => new_line_open,
    }
}

fn elem_lines(source: &SourceFile, elems: &[Node<'_>]) -> Option<(usize, usize)> {
    if elems.is_empty() {
        return None;
    }
    let first_line = shared::node_line(source, elems[0]);
    let (last_line, _) =
        source.offset_to_line_col(elems.last().unwrap().end_byte().saturating_sub(1));
    Some((first_line, last_line))
}

fn open_close_lines(source: &SourceFile, node: Node<'_>) -> Option<(usize, usize)> {
    let open_line = shared::node_line(source, node);
    let (close_line, _) = source.offset_to_line_col(node.end_byte() - 1);
    if open_line == close_line {
        None
    } else {
        Some((open_line, close_line))
    }
}

fn last_elem_end_with_comma(bytes: &[u8], last: Node<'_>) -> usize {
    let mut i = last.end_byte();
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t') {
        i += 1;
    }
    if bytes.get(i) == Some(&b',') {
        i + 1
    } else {
        last.end_byte()
    }
}

fn line_has_comment(bytes: &[u8], line_start: usize) -> bool {
    let mut i = line_start;
    while i < bytes.len() && bytes[i] != b'\n' {
        if bytes[i] == b'#' {
            return true;
        }
        i += 1;
    }
    false
}

fn left_space_start(bytes: &[u8], close_off: usize) -> usize {
    let mut i = close_off;
    while i > 0 && matches!(bytes[i - 1], b' ' | b'\t' | b'\n' | b'\r') {
        i -= 1;
    }
    i
}

fn insert_nl_before_close(cop: &dyn Cop, close_off: usize, corr: &mut Vec<Correction>) {
    corr.push(Correction {
        start: close_off,
        end: close_off,
        replacement: "\n".into(),
        cop_name: cop.name(),
        cop_index: 0,
        owner: None,
    });
}

fn move_close_inline(
    cop: &dyn Cop,
    source: &SourceFile,
    bytes: &[u8],
    last: Node<'_>,
    close_off: usize,
    close_ch: &str,
    corr: &mut Vec<Correction>,
) -> bool {
    let (last_line, _) = source.offset_to_line_col(last.end_byte().saturating_sub(1));
    if let Some(ls) = source.line_start(last_line) {
        if line_has_comment(bytes, ls) {
            return false;
        }
    }
    let insert_at = last_elem_end_with_comma(bytes, last);
    let remove_start = left_space_start(bytes, close_off);
    if insert_at > remove_start {
        return false;
    }
    corr.push(Correction {
        start: insert_at,
        end: insert_at,
        replacement: close_ch.to_string(),
        cop_name: cop.name(),
        cop_index: 0,
        owner: None,
    });
    corr.push(Correction {
        start: remove_start,
        end: close_off + 1,
        replacement: String::new(),
        cop_name: cop.name(),
        cop_index: 1,
        owner: None,
    });
    true
}

fn correct_braces(
    cop: &dyn Cop,
    source: &SourceFile,
    node: Node<'_>,
    elems: &[Node<'_>],
    want_nl_close: bool,
    close_on_same: bool,
    corrections: &mut Option<&mut Vec<Correction>>,
) -> bool {
    let Some(corr) = corrections.as_deref_mut() else {
        return false;
    };
    let bytes = source.as_bytes();
    let close_off = node.end_byte() - 1;
    if want_nl_close && close_on_same {
        insert_nl_before_close(cop, close_off, corr);
        return true;
    }
    if !want_nl_close && !close_on_same {
        return move_close_inline(
            cop,
            source,
            bytes,
            *elems.last().unwrap(),
            close_off,
            &String::from_utf8_lossy(&bytes[close_off..close_off + 1]),
            corr,
        );
    }
    false
}

fn brace_message(style: &str, new_line_open: bool, method_call: bool) -> String {
    if !method_call {
        return format!("Opening and closing braces must follow EnforcedStyle `{style}`.");
    }
    match style {
        "new_line" => {
            "Closing method call brace must be on the line after the last argument.".into()
        }
        "same_line" => {
            "Closing method call brace must be on the same line as the last argument.".into()
        }
        _ if new_line_open => {
            "Closing method call brace must be on the line after the last argument when opening brace is on a separate line from the first argument.".into()
        }
        _ => {
            "Closing method call brace must be on the same line as the last argument when opening brace is on the same line as the first argument.".into()
        }
    }
}

fn report_bad_style(
    cop: &dyn Cop,
    source: &SourceFile,
    node: Node<'_>,
    elems: &[Node<'_>],
    style: &str,
    new_line_open: bool,
    want_nl_close: bool,
    close_on_same: bool,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    // RuboCop highlights `node.loc.end` (the closing brace).
    let (l, c) = source.offset_to_line_col(node.end_byte().saturating_sub(1));
    let method_call = source.as_bytes().get(node.start_byte()) == Some(&b'(');
    let mut diag = cop.diagnostic(
        source,
        l,
        c,
        brace_message(style, new_line_open, method_call),
    );
    if correct_braces(
        cop,
        source,
        node,
        elems,
        want_nl_close,
        close_on_same,
        corrections,
    ) {
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

fn named_elems<'a>(node: Node<'a>) -> Vec<Node<'a>> {
    let mut cur = node.walk();
    node.named_children(&mut cur)
        .filter(|n| n.kind() != "comment")
        .collect()
}

/// Check open/close delimiter placement against EnforcedStyle.
pub fn check_braces(
    cop: &dyn Cop,
    source: &SourceFile,
    node: Node<'_>,
    open_byte: u8,
    style: &str,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    if source.as_bytes().get(node.start_byte()) != Some(&open_byte) {
        return;
    }
    let Some((open_line, close_line)) = open_close_lines(source, node) else {
        return;
    };
    let elems = named_elems(node);
    let Some((first_line, last_line)) = elem_lines(source, &elems) else {
        return;
    };
    let new_line_open = first_line > open_line;
    let new_line_close = close_line > last_line;
    if style_ok(style, new_line_open, new_line_close) {
        return;
    }
    report_bad_style(
        cop,
        source,
        node,
        &elems,
        style,
        new_line_open,
        want_new_line_close(style, new_line_open),
        !new_line_close,
        diagnostics,
        corrections,
    );
}
