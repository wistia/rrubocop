//! Shared per-item line-break checks for Layout/Multiline*LineBreaks cops.

use tree_sitter::Node;

use crate::cop::Cop;
use crate::cop::shared;
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// First and last node of one element (a braceless hash before `&blk` spans several pairs).
type Elem<'a> = (Node<'a>, Node<'a>);

fn report_same_line(
    cop: &dyn Cop,
    source: &SourceFile,
    item: Node<'_>,
    message: &str,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (line, col) = source.offset_to_line_col(item.start_byte());
    let mut diag = cop.diagnostic(source, line, col, message.into());
    if let Some(corr) = corrections {
        corr.push(Correction {
            start: item.start_byte(),
            end: item.start_byte(),
            replacement: "\n".into(),
            cop_name: cop.name(),
            cop_index: 0,
            owner: None,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

fn elem_end_line(source: &SourceFile, n: Node<'_>) -> usize {
    source.offset_to_line_col(n.end_byte().saturating_sub(1)).0
}

fn is_hash_item(node: Node<'_>) -> bool {
    matches!(node.kind(), "pair" | "hash_splat_argument")
}

/// Named children as elements; a heredoc ends on its opener line, as in RuboCop.
fn named_elems<'a>(node: Node<'a>) -> Vec<Elem<'a>> {
    let mut cur = node.walk();
    group_hash(
        node.named_children(&mut cur)
            .filter(|n| !matches!(n.kind(), "comment" | "heredoc_body"))
            .collect(),
    )
}

/// RuboCop splits a braceless hash into pairs only when it is the last argument, so pairs
/// before a trailing `&blk` stay one element.
fn group_hash<'a>(children: Vec<Node<'a>>) -> Vec<Elem<'a>> {
    let block_last = children
        .last()
        .is_some_and(|n| n.kind() == "block_argument");
    let mut elems: Vec<Elem<'a>> = Vec::new();
    for child in children {
        match elems.last_mut() {
            Some((_, last)) if block_last && is_hash_item(*last) && is_hash_item(child) => {
                *last = child
            }
            _ => elems.push((child, child)),
        }
    }
    elems
}

/// RuboCop `MultilineElementLineBreaks#all_on_same_line?`.
fn all_on_same_line(source: &SourceFile, elems: &[Elem<'_>], ignore_last: bool) -> bool {
    let (last_first, last_end) = elems[elems.len() - 1];
    let end = if ignore_last {
        shared::node_line(source, last_first)
    } else {
        elem_end_line(source, last_end)
    };
    shared::node_line(source, elems[0].0) == end
}

fn scan_breaks(
    cop: &dyn Cop,
    source: &SourceFile,
    elems: &[Elem<'_>],
    message: &str,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let mut last_seen = 0usize;
    let mut seen = false;
    for (first, last) in elems {
        if seen && last_seen >= shared::node_line(source, *first) {
            report_same_line(cop, source, *first, message, diagnostics, corrections);
        } else {
            last_seen = elem_end_line(source, *last);
            seen = true;
        }
    }
}

/// Require each named child on its own line when the parent spans lines.
pub fn check_breaks(
    cop: &dyn Cop,
    source: &SourceFile,
    node: Node<'_>,
    message: &str,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    check_breaks_cfg(cop, source, node, message, diagnostics, corrections, false);
}

/// When `allow_multiline_final`, RuboCop `AllowMultilineFinalElement`: if the first and
/// last elements start on the same line, do not flag (`foo(a, b, { … })`).
pub fn check_breaks_cfg(
    cop: &dyn Cop,
    source: &SourceFile,
    node: Node<'_>,
    message: &str,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
    allow_multiline_final: bool,
) {
    let elems = named_elems(node);
    if elems.len() < 2 || all_on_same_line(source, &elems, allow_multiline_final) {
        return;
    }
    scan_breaks(cop, source, &elems, message, diagnostics, corrections);
}
