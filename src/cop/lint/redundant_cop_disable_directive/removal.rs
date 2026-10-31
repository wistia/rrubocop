//! Byte ranges and corrections for removing redundant disable comments.

use std::collections::HashMap;

use crate::cop::Cop;
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

use super::RedundantCopDisableDirective;
use super::directives::{cop_token_column, disable_marker, nth_cop_token};

fn line_bytes(source: &SourceFile, line_no: usize) -> Option<&[u8]> {
    let start = source.line_start(line_no)?;
    let bytes = source.as_bytes();
    let line = match bytes[start..].iter().position(|&b| b == b'\n') {
        Some(p) => &bytes[start..start + p],
        None => &bytes[start..],
    };
    Some(trim_eol(line))
}

fn trim_eol(slice: &[u8]) -> &[u8] {
    if slice.last() == Some(&b'\r') {
        &slice[..slice.len() - 1]
    } else {
        slice
    }
}

fn comment_only_span(source: &SourceFile, line_start: usize, line: &[u8]) -> (usize, usize) {
    let end =
        line_start + line.len() + usize::from(line_start + line.len() < source.as_bytes().len());
    (line_start, end)
}

fn inline_span(
    source: &SourceFile,
    line_start: usize,
    pos: usize,
    line_len: usize,
) -> (usize, usize) {
    let mut remove_from = line_start + pos;
    while remove_from > line_start && matches!(source.as_bytes()[remove_from - 1], b' ' | b'\t') {
        remove_from -= 1;
    }
    (remove_from, line_start + line_len)
}

fn disable_line(source: &SourceFile, line_no: usize) -> Option<(usize, &[u8], &str)> {
    let line_start = source.line_start(line_no)?;
    let line = line_bytes(source, line_no)?;
    Some((line_start, line, std::str::from_utf8(line).ok()?))
}

fn entire_disable_span(
    source: &SourceFile,
    line_start: usize,
    line: &[u8],
    line_str: &str,
) -> Option<(usize, usize)> {
    if line_str.trim_start().starts_with('#') {
        return Some(comment_only_span(source, line_start, line));
    }
    let pos = line_str.to_ascii_lowercase().find("# rubocop:disable")?;
    Some(inline_span(source, line_start, pos, line.len()))
}

fn entire_disable_removal_range(source: &SourceFile, line_no: usize) -> Option<(usize, usize)> {
    let (line_start, line, line_str) = disable_line(source, line_no)?;
    entire_disable_span(source, line_start, line, line_str)
}

fn ws_comma_right(line: &str, mut e: usize) -> usize {
    while e < line.len() && line.as_bytes()[e].is_ascii_whitespace() {
        e += 1;
    }
    if e < line.len() && line.as_bytes()[e] == b',' {
        e += 1;
    }
    while e < line.len() && line.as_bytes()[e].is_ascii_whitespace() {
        e += 1;
    }
    e
}

fn partial_token_span(line: &str, byte: usize, len: usize) -> (usize, usize) {
    let mut left = byte;
    while left > 0 && line.as_bytes()[left - 1].is_ascii_whitespace() {
        left -= 1;
    }
    if left > 0 && line.as_bytes()[left - 1] == b',' {
        return (left - 1, byte + len);
    }
    (byte, ws_comma_right(line, byte + len))
}

fn partial_cop_removal_range(
    source: &SourceFile,
    line_no: usize,
    cop: &str,
    occurrence: usize,
) -> Option<(usize, usize)> {
    let line_start = source.line_start(line_no)?;
    let line_str = std::str::from_utf8(line_bytes(source, line_no)?).ok()?;
    disable_marker(line_str)?;
    nth_cop_token(line_str, cop, occurrence).map(|(s, e)| {
        let (rs, re) = partial_token_span(line_str, s, e - s);
        (line_start + rs, line_start + re)
    })
}

pub(super) fn push_removal(
    source: &SourceFile,
    line_no: usize,
    cop: &str,
    occurrence: Option<usize>,
    corrections: Option<&mut Vec<Correction>>,
    diag: &mut Diagnostic,
) {
    let Some(corr) = corrections else {
        return;
    };
    let range = if let Some(n) = occurrence {
        partial_cop_removal_range(source, line_no, cop, n)
    } else {
        entire_disable_removal_range(source, line_no)
    };
    let Some((start, end)) = range else {
        return;
    };
    corr.push(Correction {
        start,
        end,
        replacement: String::new(),
        cop_name: "Lint/RedundantCopDisableDirective",
        cop_index: 0,
        owner: None,
    });
    diag.corrected = true;
}

pub(super) fn report_duplicate_cops(
    cop: &RedundantCopDisableDirective,
    source: &SourceFile,
    line_no: usize,
    name: &str,
    occurrence: usize,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let mut diag = cop.diagnostic(
        source,
        line_no,
        cop_token_column(source.line_text(line_no).unwrap_or(""), name, occurrence),
        format!("Unnecessary disabling of `{name}`."),
    );
    push_removal(
        source,
        line_no,
        name,
        Some(occurrence),
        corrections.as_deref_mut(),
        &mut diag,
    );
    diagnostics.push(diag);
}

pub(super) fn scan_duplicate_cops(
    cop: &RedundantCopDisableDirective,
    source: &SourceFile,
    line_no: usize,
    cops: &[String],
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let mut nth = HashMap::<&str, usize>::new();
    for name in cops {
        let n = nth.entry(name.as_str()).or_insert(0);
        *n += 1;
        if *n == 1 {
            continue;
        }
        report_duplicate_cops(cop, source, line_no, name, *n, diagnostics, corrections);
    }
}
