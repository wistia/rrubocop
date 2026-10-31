//! Layout/ClosingParenthesisIndentation.

use tree_sitter::Node;

use crate::cop::layout::report;
use crate::cop::shared;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ClosingParenthesisIndentation;

fn is_paren_wrapped(bytes: &[u8], node: Node<'_>) -> bool {
    bytes.get(node.start_byte()) == Some(&b'(')
        && bytes.get(node.end_byte().saturating_sub(1)) == Some(&b')')
}

fn named_elements(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cur = node.walk();
    node.named_children(&mut cur)
        .filter(|n| n.kind() != "comment")
        .collect()
}

/// RuboCop `all_elements_aligned?`: every element's start column is the same
/// (for a leading hash, compare its pair keys instead).
fn all_elements_aligned(source: &SourceFile, elements: &[Node<'_>]) -> bool {
    let cols = element_start_cols(source, elements);
    let Some(c0) = cols.first() else {
        return true;
    };
    cols.iter().all(|c| c == c0)
}

fn element_start_cols(source: &SourceFile, elements: &[Node<'_>]) -> Vec<usize> {
    let Some(first) = elements.first() else {
        return Vec::new();
    };
    if first.kind() == "hash" {
        let mut cur = first.walk();
        return first
            .named_children(&mut cur)
            .filter(|n| n.kind() == "pair")
            .map(|n| shared::node_col(source, n))
            .collect();
    }
    elements
        .iter()
        .map(|n| shared::node_col(source, *n))
        .collect()
}

/// RuboCop `expected_column` for a hanging `)`.
fn expected_close_col(source: &SourceFile, node: Node<'_>, indent_width: usize) -> usize {
    if node.parent().is_some_and(|n| n.kind() == "return") {
        return shared::line_indent(source, node.start_byte());
    }
    expected_argument_close_col(source, node, indent_width)
}

fn expected_argument_close_col(source: &SourceFile, node: Node<'_>, indent_width: usize) -> usize {
    let open_line = shared::node_line(source, node);
    let open_col = shared::node_col(source, node);
    let elements = named_elements(node);
    let Some(first) = elements.first().copied() else {
        // empty multiline `()` → align `)` with `(`
        return open_col;
    };
    if shared::node_line(source, first) > open_line {
        // First arg on next line → `)` outdented by IndentationWidth from that indent.
        return shared::line_indent(source, first.start_byte()).saturating_sub(indent_width);
    }
    if all_elements_aligned(source, &elements) {
        open_col
    } else {
        // Params not lined up — outdent `)` to the first argument's line indent.
        shared::line_indent(source, first.start_byte())
    }
}

impl Cop for ClosingParenthesisIndentation {
    fn name(&self) -> &'static str {
        "Layout/ClosingParenthesisIndentation"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &[
            "argument_list",
            "parenthesized_statements",
            "method_parameters",
        ]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if !is_paren_wrapped(source.as_bytes(), node) {
            return;
        }
        let open_line = shared::node_line(source, node);
        let close_off = node.end_byte() - 1;
        let (close_line, close_col) = source.offset_to_line_col(close_off);
        if open_line == close_line {
            return;
        }
        // Only hanging `)` (starts its line).
        if shared::line_indent(source, close_off) != close_col {
            return;
        }
        let want = expected_close_col(source, node, config.get_usize("IndentationWidth", 2));
        if close_col == want {
            return;
        }
        report::fix_indent(
            self,
            source,
            close_off,
            format!("Indent `)` to column {want}."),
            diagnostics,
            &mut corrections,
            shared::line_indent(source, close_off),
            want,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        ClosingParenthesisIndentation,
        "cops/layout/closing_parenthesis_indentation"
    );
}
