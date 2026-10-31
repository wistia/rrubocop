//! Layout/CommentIndentation.

use tree_sitter::{Node, Tree};

use crate::cop::layout::report;
use crate::cop::shared;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct CommentIndentation;

fn standalone_comment(bytes: &[u8], ls: usize, start: usize) -> bool {
    !bytes[ls..start].iter().any(|&b| b != b' ' && b != b'\t')
}

fn line_body(bytes: &[u8], nls: usize) -> &[u8] {
    &bytes[nls..bytes[nls..]
        .iter()
        .position(|&b| b == b'\n')
        .map(|i| nls + i)
        .unwrap_or(bytes.len())]
}

fn content_indent(line: &[u8]) -> Option<usize> {
    let i = line
        .iter()
        .take_while(|&&b| matches!(b, b' ' | b'\t'))
        .count();
    (i < line.len() && line[i] != b'#').then_some(i)
}

fn stripped(line: &[u8]) -> &[u8] {
    &line[line
        .iter()
        .take_while(|&&b| matches!(b, b' ' | b'\t'))
        .count()..]
}

fn less_indented(line: &[u8]) -> bool {
    let rest = stripped(line);
    (rest.starts_with(b"end")
        && rest
            .get(3)
            .is_none_or(|b| !b.is_ascii_alphanumeric() && *b != b'_'))
        || rest
            .first()
            .is_some_and(|b| matches!(b, b'}' | b')' | b']'))
}

fn two_alternatives(line: &[u8]) -> bool {
    let rest = stripped(line);
    rest.starts_with(b"else")
        || rest.starts_with(b"elsif")
        || rest.starts_with(b"when")
        || rest.starts_with(b"in ")
        || rest.starts_with(b"rescue")
        || rest.starts_with(b"ensure")
}

fn next_code_line<'a>(source: &SourceFile, bytes: &'a [u8], line: usize) -> Option<&'a [u8]> {
    (line + 1..=line + 40).find_map(|ln| {
        let nls = source.line_start(ln)?;
        let body = line_body(bytes, nls);
        content_indent(body).map(|_| body)
    })
}

fn expected_col(next: &[u8], width: usize) -> usize {
    let base = content_indent(next).unwrap_or(0);
    if less_indented(next) {
        base + width
    } else {
        base
    }
}

fn check_comment(
    cop: &dyn Cop,
    source: &SourceFile,
    bytes: &[u8],
    comment: Node<'_>,
    width: usize,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let Some((start, col, ls, expected)) = bad_indent(source, bytes, comment, width) else {
        return;
    };
    report::report_fix(
        cop,
        source,
        start,
        format!("Incorrect indentation detected (column {col} instead of {expected})."),
        diagnostics,
        corrections,
        ls,
        start,
        " ".repeat(expected),
    );
}

/// Standalone `#` comment site: (start, line, col, line_start).
fn hash_comment_site(
    source: &SourceFile,
    bytes: &[u8],
    comment: Node<'_>,
) -> Option<(usize, usize, usize, usize)> {
    let start = comment.start_byte();
    if bytes.get(start) != Some(&b'#') {
        return None;
    }
    let (line, col) = source.offset_to_line_col(start);
    let ls = source.line_start(line).unwrap_or(0);
    standalone_comment(bytes, ls, start).then_some((start, line, col, ls))
}

fn bad_indent(
    source: &SourceFile,
    bytes: &[u8],
    comment: Node<'_>,
    width: usize,
) -> Option<(usize, usize, usize, usize)> {
    let (start, line, col, ls) = hash_comment_site(source, bytes, comment)?;
    let next = next_code_line(source, bytes, line)?;
    let expected = expected_col(next, width);
    let ok = col == expected || (two_alternatives(next) && col == expected + width);
    (!ok).then_some((start, col, ls, expected))
}

impl Cop for CommentIndentation {
    fn name(&self) -> &'static str {
        "Layout/CommentIndentation"
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
        let width = config.get_usize("Width", 2);
        let bytes = source.as_bytes();
        for comment in shared::collect_comments(tree.root_node()) {
            check_comment(
                self,
                source,
                bytes,
                comment,
                width,
                diagnostics,
                &mut corrections,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CommentIndentation, "cops/layout/comment_indentation");
}
