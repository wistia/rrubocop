//! Shared diagnostic + correction helpers for layout cops.

use tree_sitter::Node;

use crate::cop::Cop;
use crate::cop::shared;
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

/// RuboCop `AlignmentCorrector`: shift every line of `node` by `delta` columns,
/// except blank lines and lines where `skip(line_start)` holds (string / heredoc interiors).
pub fn shift_lines(
    cop: &dyn Cop,
    source: &SourceFile,
    node: Node<'_>,
    delta: isize,
    skip: impl Fn(usize) -> bool,
    corrections: &mut Vec<Correction>,
) -> bool {
    let end_line = source
        .offset_to_line_col(node.end_byte().saturating_sub(1))
        .0;
    let mut applied = false;
    for line in shared::node_line(source, node)..=end_line {
        let Some(start) = source.line_start(line) else {
            continue;
        };
        let indent = shared::line_indent(source, start);
        let new_indent = (indent as isize + delta).max(0) as usize;
        let blank = matches!(
            source.as_bytes().get(start + indent),
            None | Some(b'\n' | b'\r')
        );
        if new_indent == indent || blank || skip(start) {
            continue;
        }
        corrections.push(Correction {
            start,
            end: start + indent,
            replacement: " ".repeat(new_indent),
            cop_name: cop.name(),
            cop_index: 0,
            owner: None,
        });
        applied = true;
    }
    applied
}

pub fn report_fix(
    cop: &dyn Cop,
    source: &SourceFile,
    off: usize,
    msg: String,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
    start: usize,
    end: usize,
    replacement: String,
) {
    let (line, col) = source.offset_to_line_col(off);
    let mut diag = cop.diagnostic(source, line, col, msg);
    if let Some(corr) = corrections {
        corr.push(Correction {
            start,
            end,
            replacement,
            cop_name: cop.name(),
            cop_index: 0,
            owner: None,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

pub fn insert_space(
    cop: &dyn Cop,
    source: &SourceFile,
    off: usize,
    msg: String,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
    at: usize,
) {
    report_fix(
        cop,
        source,
        off,
        msg,
        diagnostics,
        corrections,
        at,
        at,
        " ".into(),
    );
}

pub fn insert_newline(
    cop: &dyn Cop,
    source: &SourceFile,
    line: usize,
    msg: String,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let mut diag = cop.diagnostic(source, line, 0, msg);
    if let Some(corr) = corrections {
        if let Some(offset) = source.line_start(line) {
            corr.push(Correction {
                start: offset,
                end: offset,
                replacement: "\n".into(),
                cop_name: cop.name(),
                cop_index: 0,
                owner: None,
            });
            diag.corrected = true;
        }
    }
    diagnostics.push(diag);
}

pub fn fix_indent(
    cop: &dyn Cop,
    source: &SourceFile,
    off: usize,
    msg: String,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
    cur_indent: usize,
    expected: usize,
) {
    let (l, c) = source.offset_to_line_col(off);
    let mut diag = cop.diagnostic(source, l, c, msg);
    if let Some(corr) = corrections {
        if let Some(ls) = source.line_start(l) {
            corr.push(Correction {
                start: ls,
                end: ls + cur_indent,
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
