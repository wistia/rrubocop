//! Shared trailing-comma source-line geometry.

use crate::parse::source::SourceFile;

pub(crate) fn begins_its_line(source: &SourceFile, offset: usize) -> bool {
    crate::cop::shared::line_indent(source, offset) == source.offset_to_line_col(offset).1
}

fn end_line(source: &SourceFile, start: usize, end: usize) -> usize {
    source
        .offset_to_line_col(end.saturating_sub(1).max(start))
        .0
}

/// RuboCop `no_elements_on_same_line?`.
pub(crate) fn each_elem_own_line(
    source: &SourceFile,
    locs: &[(usize, usize)],
    close: usize,
) -> bool {
    for w in locs.windows(2) {
        if end_line(source, w[0].0, w[0].1) == source.offset_to_line_col(w[1].0).0 {
            return false;
        }
    }
    match locs.last() {
        Some(&(s, e)) if end_line(source, s, e) == source.offset_to_line_col(close).0 => false,
        Some(_) => true,
        None => false,
    }
}

pub(crate) fn last_item_line_start(source: &SourceFile, start: usize, end: usize) -> usize {
    let bytes = &source.as_bytes()[start..end];
    let last = bytes
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |position| position + 1);
    start
        + last
        + bytes[last..]
            .iter()
            .position(|byte| !byte.is_ascii_whitespace())
            .unwrap_or(0)
}
