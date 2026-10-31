//! Layout/EmptyLineBetweenDefs.

use tree_sitter::{Node, Tree};

use crate::cop::shared;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct EmptyLineBetweenDefs;

fn is_method(n: Node<'_>) -> bool {
    matches!(n.kind(), "method" | "singleton_method")
}

fn expected_label(number: usize) -> String {
    if number == 1 {
        "1 empty line".into()
    } else {
        format!("{number} empty lines")
    }
}

fn fix_gap(
    cop: &dyn Cop,
    source: &SourceFile,
    a_end: usize,
    b_start: usize,
    number: usize,
    corr: &mut Vec<Correction>,
) -> bool {
    let Some(insert_at) = source.line_start(a_end + 1) else {
        return false;
    };
    corr.push(Correction {
        start: insert_at,
        end: source.line_start(b_start).unwrap_or(insert_at),
        replacement: "\n".repeat(number),
        cop_name: cop.name(),
        cop_index: 0,
        owner: None,
    });
    true
}

fn is_blank_line(source: &SourceFile, line: usize) -> bool {
    let Some(start) = source.line_start(line) else {
        return false;
    };
    let bytes = source.as_bytes();
    let mut i = start;
    while i < bytes.len() && bytes[i] != b'\n' {
        if !matches!(bytes[i], b' ' | b'\t' | b'\r') {
            return false;
        }
        i += 1;
    }
    true
}

fn gap_blanks(source: &SourceFile, a: Node<'_>, b: Node<'_>) -> (usize, usize, usize) {
    let (a_end, _) = source.offset_to_line_col(a.end_byte().saturating_sub(1));
    let b_start = shared::node_line(source, b);
    // RuboCop counts only blank lines between defs (comments are not blank).
    let mut blanks = 0usize;
    let mut line = a_end + 1;
    while line < b_start {
        if is_blank_line(source, line) {
            blanks += 1;
        }
        line += 1;
        if blanks > 100 {
            break;
        }
    }
    (a_end, b_start, blanks)
}

/// RuboCop `multiple_blank_lines_groups?` — extra blanks OK when comments separate groups.
fn multiple_blank_groups(source: &SourceFile, a_end: usize, b_start: usize) -> bool {
    if b_start <= a_end + 1 {
        return false;
    }
    let mut blank_idxs = Vec::new();
    let mut non_blank_idxs = Vec::new();
    for line in (a_end + 1)..b_start {
        if is_blank_line(source, line) {
            blank_idxs.push(line);
        } else {
            non_blank_idxs.push(line);
        }
    }
    let Some(&blank_start) = blank_idxs.iter().max() else {
        return false;
    };
    let Some(&non_blank_end) = non_blank_idxs.iter().min() else {
        return false;
    };
    blank_start > non_blank_end
}

fn one_line_def(source: &SourceFile, n: Node<'_>) -> bool {
    shared::node_line(source, n) == source.offset_to_line_col(n.end_byte().saturating_sub(1)).0
}

fn report_gap(
    cop: &dyn Cop,
    source: &SourceFile,
    b: Node<'_>,
    a_end: usize,
    b_start: usize,
    number: usize,
    blanks: usize,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (line, col) = source.offset_to_line_col(b.start_byte());
    let expected = expected_label(number);
    let mut diag = cop.diagnostic(
        source,
        line,
        col,
        format!("Expected {expected} between method definitions; found {blanks}."),
    );
    if let Some(corr) = corrections {
        if fix_gap(cop, source, a_end, b_start, number, corr) {
            diag.corrected = true;
        }
    }
    diagnostics.push(diag);
}

fn check_pair(
    cop: &dyn Cop,
    source: &SourceFile,
    a: Node<'_>,
    b: Node<'_>,
    number: usize,
    allow_adjacent_one_line: bool,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    if !is_method(a) || !is_method(b) {
        return;
    }
    let (a_end, b_start, blanks) = gap_blanks(source, a, b);
    if blanks == number {
        return;
    }
    if multiple_blank_groups(source, a_end, b_start) {
        return;
    }
    if allow_adjacent_one_line && one_line_def(source, a) && one_line_def(source, b) {
        return;
    }
    report_gap(
        cop,
        source,
        b,
        a_end,
        b_start,
        number,
        blanks,
        diagnostics,
        corrections,
    );
}

/// RuboCop `on_begin`: only consecutive sibling pairs that are both defs.
fn check_body(
    cop: &dyn Cop,
    source: &SourceFile,
    body: Node<'_>,
    number: usize,
    allow_adjacent_one_line: bool,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let mut cur = body.walk();
    let kids: Vec<_> = body
        .named_children(&mut cur)
        .filter(|n| n.kind() != "comment")
        .collect();
    for w in kids.windows(2) {
        check_pair(
            cop,
            source,
            w[0],
            w[1],
            number,
            allow_adjacent_one_line,
            diagnostics,
            corrections,
        );
    }
}

impl Cop for EmptyLineBetweenDefs {
    fn name(&self) -> &'static str {
        "Layout/EmptyLineBetweenDefs"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn uses_source_phase(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        tree: &Tree,
        _code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let number = config.get_usize("NumberOfEmptyLines", 1);
        let allow_adjacent = config.get_bool("AllowAdjacentOneLineDefs", true);
        shared::for_each_descendant(tree.root_node(), |n| {
            if matches!(n.kind(), "body_statement" | "begin" | "program") {
                check_body(
                    self,
                    source,
                    n,
                    number,
                    allow_adjacent,
                    diagnostics,
                    &mut corrections,
                );
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(EmptyLineBetweenDefs, "cops/layout/empty_line_between_defs");
}
