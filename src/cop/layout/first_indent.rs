//! Shared first-child indentation for Layout/First*Indentation cops.

use tree_sitter::Node;

use crate::cop::Cop;
use crate::cop::shared;
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub(crate) fn argument_list_opens_with_paren(node: Node<'_>) -> bool {
    node.kind() == "argument_list"
        && node
            .children(&mut node.walk())
            .any(|c| !c.is_named() && c.kind() == "(")
}

/// RuboCop `special_inside_parentheses`: only `{`/`[` literals that begin a
/// parenthesized argument list (not keyword-hash values like `locals: {}`).
fn special_inside_parentheses_target(node: Node<'_>) -> bool {
    if node.parent().is_some_and(|p| p.kind() == "pair") {
        return false;
    }
    let Some(parent) = node.parent() else {
        return false;
    };
    if !argument_list_opens_with_paren(parent) {
        return false;
    }
    parent
        .named_children(&mut parent.walk())
        .next()
        .is_some_and(|first| first == node)
}

fn style_applies(node: Node<'_>, style: &str) -> bool {
    if style.starts_with("special_for_") {
        return false;
    }
    if style == "special_inside_parentheses" {
        return special_inside_parentheses_target(node);
    }
    true
}

fn report_indent(
    cop: &dyn Cop,
    source: &SourceFile,
    first: Node<'_>,
    actual: usize,
    expected: usize,
    message: String,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (l, c) = source.offset_to_line_col(first.start_byte());
    let mut diag = cop.diagnostic(source, l, c, message);
    if let Some(corr) = corrections {
        if let Some(ls) = source.line_start(l) {
            corr.push(Correction {
                start: ls,
                end: ls + actual,
                replacement: " ".repeat(expected),
                cop_name: cop.name(),
                cop_index: 0,
                owner: None,
            });
            diag.corrected = true;
        }
    }
    diagnostics.push(diag);
}

fn expected_col(source: &SourceFile, node: Node<'_>, width: usize, style: &str) -> usize {
    match style {
        "consistent" => shared::line_indent(source, node.start_byte()) + width,
        _ => shared::node_col(source, node) + width,
    }
}

fn same_indent_multiline_span(source: &SourceFile, elems: &[Node<'_>], start_line: usize) -> bool {
    if elems.len() < 2 {
        return false;
    }
    let last = elems[elems.len() - 1];
    shared::node_line(source, last) != start_line
        && shared::line_indent(source, elems[0].start_byte())
            == shared::line_indent(source, last.start_byte())
}

fn skip_first_indent(source: &SourceFile, elems: &[Node<'_>], start_line: usize) -> bool {
    shared::node_line(source, elems[0]) == start_line
        || (elems.len() > 1 && same_indent_multiline_span(source, elems, start_line))
}

/// Check that the first named child of a multiline construct is indented by `width`.
pub fn check_first(
    cop: &dyn Cop,
    source: &SourceFile,
    node: Node<'_>,
    width: usize,
    style: &str,
    message: String,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    if !style_applies(node, style) {
        return;
    }
    let mut cur = node.walk();
    let elems: Vec<_> = node.named_children(&mut cur).collect();
    if elems.is_empty() {
        return;
    }
    if skip_first_indent(source, &elems, shared::node_line(source, node)) {
        return;
    }
    let first = elems[0];
    let expected = expected_col(source, node, width, style);
    let actual = shared::line_indent(source, first.start_byte());
    if actual != expected {
        report_indent(
            cop,
            source,
            first,
            actual,
            expected,
            message,
            diagnostics,
            corrections,
        );
    }
}
