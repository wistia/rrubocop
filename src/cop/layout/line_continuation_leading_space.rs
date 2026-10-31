//! Layout/LineContinuationLeadingSpace — spaces inside continued string literals.

use tree_sitter::Node;

use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct LineContinuationLeadingSpace;

/// Trailing style: spaces after the opening quote on the continued line.
fn trailing_offense(second_line: &[u8]) -> Option<(usize, usize)> {
    let mut i = 0;
    while i < second_line.len() && matches!(second_line[i], b' ' | b'\t') {
        i += 1;
    }
    if i >= second_line.len() || !matches!(second_line[i], b'\'' | b'"') {
        return None;
    }
    i += 1;
    let start = i;
    while i < second_line.len() && second_line[i] == b' ' {
        i += 1;
    }
    (i > start).then_some((start, i - start))
}

/// Leading style: spaces between closing quote and `\` on the first line.
fn leading_offense(first_line: &[u8]) -> Option<(usize, usize)> {
    let quote = quote_before_backslash(first_line)?;
    let space_start = quote + 1;
    let bs = first_line.iter().rposition(|&b| b == b'\\')?;
    (bs > space_start).then_some((space_start, bs - space_start))
}

fn quote_before_backslash(line: &[u8]) -> Option<usize> {
    let bs = line.iter().rposition(|&b| b == b'\\')?;
    let mut q = bs;
    while q > 0 && matches!(line[q - 1], b' ' | b'\t' | b'\r') {
        q -= 1;
    }
    (q > 0 && matches!(line[q - 1], b'\'' | b'"')).then_some(q - 1)
}

fn opening_quote_off(line: &[u8]) -> Option<usize> {
    let mut i = 0;
    while i < line.len() && matches!(line[i], b' ' | b'\t') {
        i += 1;
    }
    (i < line.len() && matches!(line[i], b'\'' | b'"')).then_some(i)
}

fn line_ends_with_cont(line: &[u8]) -> bool {
    line.ends_with(b"\\")
}

impl Cop for LineContinuationLeadingSpace {
    fn name(&self) -> &'static str {
        "Layout/LineContinuationLeadingSpace"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["string", "chained_string"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let style = config.get_str("EnforcedStyle", "trailing");
        let text = &source.as_bytes()[node.start_byte()..node.end_byte()];
        if !text.contains(&b'\\') {
            return;
        }
        let start_line = source.offset_to_line_col(node.start_byte()).0;
        let end_line = source
            .offset_to_line_col(node.end_byte().saturating_sub(1))
            .0;
        for line_no in start_line..end_line {
            report_continuation(self, source, line_no, style, diagnostics, &mut corrections);
        }
    }
}

fn report_continuation(
    cop: &LineContinuationLeadingSpace,
    source: &SourceFile,
    line_no: usize,
    style: &str,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let Some((line_start, next_start, line, next)) = continuation_pair(source, line_no) else {
        return;
    };
    let Some((byte, len, insert_at)) = offense_fix(style, line_start, next_start, line, next)
    else {
        return;
    };
    let (l, c) = source.offset_to_line_col(byte);
    let mut diag = cop.diagnostic(
        source,
        l,
        c,
        "Do not use more than one space before a line-continued string.".to_string(),
    );
    if push_space_move(cop, source, byte, len, insert_at, corrections) {
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

fn push_space_move(
    cop: &LineContinuationLeadingSpace,
    source: &SourceFile,
    byte: usize,
    len: usize,
    insert_at: usize,
    corrections: &mut Option<&mut Vec<Correction>>,
) -> bool {
    let Some(corr) = corrections else {
        return false;
    };
    let spaces = String::from_utf8_lossy(&source.as_bytes()[byte..byte + len]).into_owned();
    corr.push(Correction {
        start: byte,
        end: byte + len,
        replacement: String::new(),
        cop_name: cop.name(),
        cop_index: 0,
        owner: None,
    });
    corr.push(Correction {
        start: insert_at,
        end: insert_at,
        replacement: spaces,
        cop_name: cop.name(),
        cop_index: 0,
        owner: None,
    });
    true
}

fn offense_fix(
    style: &str,
    line_start: usize,
    next_start: usize,
    line: &[u8],
    next: &[u8],
) -> Option<(usize, usize, usize)> {
    if style == "leading" {
        let (off, len) = leading_offense(line)?;
        let q = opening_quote_off(next)?;
        Some((line_start + off, len, next_start + q + 1))
    } else {
        let (off, len) = trailing_offense(next)?;
        let q = quote_before_backslash(line)?;
        Some((next_start + off, len, line_start + q))
    }
}

fn continuation_pair<'a>(
    source: &'a SourceFile,
    line_no: usize,
) -> Option<(usize, usize, &'a [u8], &'a [u8])> {
    let line_start = source.line_start(line_no)?;
    let next_start = source.line_start(line_no + 1)?;
    let bytes = source.as_bytes();
    let line = trim_line_ending(&bytes[line_start..next_start]);
    if !line_ends_with_cont(line) {
        return None;
    }
    Some((
        line_start,
        next_start,
        line,
        trim_line_ending(&bytes[next_start..source.line_start(line_no + 2).unwrap_or(bytes.len())]),
    ))
}

fn trim_line_ending(mut slice: &[u8]) -> &[u8] {
    if slice.last() == Some(&b'\n') {
        slice = &slice[..slice.len() - 1];
    }
    if slice.last() == Some(&b'\r') {
        slice = &slice[..slice.len() - 1];
    }
    slice
}
