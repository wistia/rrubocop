//! Shared alignment loop for Layout/*Alignment cops.

mod items;

use tree_sitter::Node;

use crate::cop::Cop;
use crate::cop::shared;
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

fn expected_col(style: &str, fixed_col: usize, base_col: usize) -> usize {
    if style == "with_fixed_indentation" {
        fixed_col
    } else {
        base_col
    }
}

fn push_indent_fix(
    corr: &mut Vec<Correction>,
    cop_name: &'static str,
    line_start: usize,
    cur_indent: usize,
    expected: usize,
) {
    corr.push(Correction {
        start: line_start,
        end: line_start + cur_indent,
        replacement: " ".repeat(expected),
        cop_name,
        cop_index: 0,
        owner: None,
    });
}

fn report_misaligned(
    cop: &dyn Cop,
    source: &SourceFile,
    item: Node<'_>,
    expected: usize,
    message: &str,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (l, c) = source.offset_to_line_col(item.start_byte());
    let mut diag = cop.diagnostic(source, l, c, message.into());
    if let Some(corr) = corrections {
        if let Some(line_start) = source.line_start(l) {
            push_indent_fix(
                corr,
                cop.name(),
                line_start,
                shared::line_indent(source, item.start_byte()),
                expected,
            );
            diag.corrected = true;
        }
    }
    diagnostics.push(diag);
}

fn check_one(
    cop: &dyn Cop,
    source: &SourceFile,
    item: Node<'_>,
    first_line: usize,
    style: &str,
    fixed_col: usize,
    base_col: usize,
    message: &str,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    if shared::node_line(source, item) == first_line {
        return;
    }
    if shared::node_col(source, item) != shared::line_indent(source, item.start_byte()) {
        return;
    }
    let expected = expected_col(style, fixed_col, base_col);
    if shared::node_col(source, item) != expected {
        report_misaligned(
            cop,
            source,
            item,
            expected,
            message,
            diagnostics,
            corrections,
        );
    }
}

fn align_cols(
    source: &SourceFile,
    node: Node<'_>,
    items: &[Node<'_>],
    width: usize,
    style: &str,
) -> (usize, usize, usize) {
    let first_line = shared::node_line(source, items[0]);
    let base_col = shared::node_col(source, items[0]);
    let fixed_col = if style == "with_fixed_indentation" {
        if node.kind() == "array" {
            shared::line_indent(source, node.start_byte()) + width
        } else {
            selector_line_indent(source, node) + width
        }
    } else {
        shared::line_indent(
            source,
            node.parent()
                .filter(|p| matches!(p.kind(), "call" | "command" | "command_call"))
                .unwrap_or(node)
                .start_byte(),
        ) + width
    };
    (first_line, base_col, fixed_col)
}

fn call_selector_indent(source: &SourceFile, call: Node<'_>, arg_list: Node<'_>) -> usize {
    if let Some(method) = call.child_by_field_name("method") {
        return shared::line_indent(source, method.start_byte());
    }
    let bytes = source.as_bytes();
    let from = call.start_byte();
    if let Some(rel) = bytes[from..arg_list.start_byte().min(bytes.len())]
        .iter()
        .rposition(|&b| b == b'.' || b == b'(')
    {
        return shared::line_indent(source, from + rel);
    }
    shared::line_indent(source, arg_list.start_byte())
}

fn selector_line_indent(source: &SourceFile, arg_list: Node<'_>) -> usize {
    let Some(call) = arg_list
        .parent()
        .filter(|p| matches!(p.kind(), "call" | "command" | "command_call"))
    else {
        return shared::line_indent(source, arg_list.start_byte());
    };
    call_selector_indent(source, call, arg_list)
}

fn items_span_lines(source: &SourceFile, items: &[Node<'_>]) -> bool {
    let first_line = shared::node_line(source, items[0]);
    items
        .iter()
        .any(|i| shared::node_line(source, *i) != first_line)
}

/// Align named children of `node` that span multiple lines.
pub fn check_align(
    cop: &dyn Cop,
    source: &SourceFile,
    node: Node<'_>,
    style: &str,
    width: usize,
    message: &str,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    check_align_items(
        cop,
        source,
        node,
        &items::alignment_items(source, node, style),
        style,
        width,
        message,
        diagnostics,
        corrections,
    );
}

fn check_align_items(
    cop: &dyn Cop,
    source: &SourceFile,
    node: Node<'_>,
    items: &[Node<'_>],
    style: &str,
    width: usize,
    message: &str,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    if items.len() < 2 || !items_span_lines(source, items) {
        return;
    }
    let (first_line, base_col, fixed_col) = align_cols(source, node, items, width, style);
    for item in items.iter().skip(1) {
        check_one(
            cop,
            source,
            *item,
            first_line,
            style,
            fixed_col,
            base_col,
            message,
            diagnostics,
            corrections,
        );
    }
}
