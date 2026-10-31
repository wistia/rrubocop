//! Layout/ExtraSpacing.

use tree_sitter::{Node, Tree};

use crate::cop::layout::report;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::{SourceFile, utf8_byte_index_to_column};

pub struct ExtraSpacing;

fn skip_indent(line: &[u8]) -> usize {
    let mut i = 0;
    while i < line.len() && matches!(line[i], b' ' | b'\t') {
        i += 1;
    }
    i
}

fn is_full_line_comment(line: &[u8]) -> bool {
    line.get(skip_indent(line)) == Some(&b'#')
}

fn utf8_char_len(b: u8) -> usize {
    match b {
        0x00..0x80 => 1,
        0xC0..0xE0 => 2,
        0xE0..0xF0 => 3,
        0xF0..0xF8 => 4,
        _ => 1,
    }
}

fn byte_at(line: &[u8], col: usize) -> Option<u8> {
    let mut dcol = 0usize;
    let mut i = 0usize;
    while i < line.len() {
        if dcol == col {
            return line.get(i).copied();
        }
        i += utf8_char_len(line[i]);
        dcol += 1;
    }
    None
}

/// RuboCop `aligned_words?`: token begins at display `col` (`/\s\S/` at `col - 1`).
fn word_starts_at(line: &[u8], col: usize) -> bool {
    col > 0
        && matches!(byte_at(line, col - 1), Some(b' ') | Some(b'\t'))
        && byte_at(line, col).is_some_and(|b| !matches!(b, b' ' | b'\t' | b'\n' | b'\r'))
}

fn aligned_elsewhere(lines: &[&[u8]], line_idx: usize, col: usize, eq_token: bool) -> bool {
    col > 0
        && lines.iter().enumerate().any(|(other, bytes)| {
            other != line_idx
                && !is_full_line_comment(bytes)
                && (word_starts_at(bytes, col) || (eq_token && byte_at(bytes, col) == Some(b'=')))
        })
}

fn in_ignored(ignored: &[(usize, usize)], abs: usize) -> bool {
    ignored.iter().any(|&(a, b)| abs >= a && abs < b)
}

fn pair_gap(pair: Node<'_>) -> Option<(usize, usize)> {
    let key = pair.child_by_field_name("key")?;
    let value = pair.child_by_field_name("value")?;
    let (start, end) = (key.end_byte(), value.start_byte());
    (end > start).then_some((start, end))
}

fn collect_hash_gaps(node: Node<'_>, out: &mut Vec<(usize, usize)>) {
    // RuboCop ignores key/value gaps in multiline hashes (Layout/HashAlignment).
    if matches!(node.kind(), "hash" | "bare_hash" | "argument_list")
        && node.start_position().row != node.end_position().row
    {
        let mut cur = node.walk();
        for child in node.named_children(&mut cur) {
            if child.kind() == "pair" {
                if let Some(gap) = pair_gap(child) {
                    out.push(gap);
                }
            }
        }
    }
    let mut cur = node.walk();
    for child in node.children(&mut cur) {
        collect_hash_gaps(child, out);
    }
}

fn skip_run(
    lines: &[&[u8]],
    ignored: &[(usize, usize)],
    allow_aligned: bool,
    allow_before_comment: bool,
    line_idx: usize,
    line: &[u8],
    abs: usize,
    end: usize,
) -> bool {
    if in_ignored(ignored, abs) {
        return true;
    }
    let after = line.get(end).copied();
    if after == Some(b'#') && allow_before_comment {
        return true;
    }
    if !allow_aligned {
        return false;
    }
    aligned_elsewhere(
        lines,
        line_idx,
        utf8_byte_index_to_column(&line[..end]),
        after == Some(b'='),
    )
}

fn check_run(
    cop: &dyn Cop,
    source: &SourceFile,
    code_map: &CodeMap,
    lines: &[&[u8]],
    ignored: &[(usize, usize)],
    allow_aligned: bool,
    allow_before_comment: bool,
    line_idx: usize,
    offset: usize,
    line: &[u8],
    start: usize,
    end: usize,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let run = end - start;
    if run < 2 {
        return;
    }
    let abs = offset + start;
    if code_map.covers(abs)
        || skip_run(
            lines,
            ignored,
            allow_aligned,
            allow_before_comment,
            line_idx,
            line,
            abs,
            end,
        )
    {
        return;
    }
    report::report_fix(
        cop,
        source,
        abs,
        "Unnecessary spacing detected.".into(),
        diagnostics,
        corrections,
        abs,
        abs + run,
        " ".into(),
    );
}

fn scan_line(
    cop: &dyn Cop,
    source: &SourceFile,
    code_map: &CodeMap,
    lines: &[&[u8]],
    ignored: &[(usize, usize)],
    allow_aligned: bool,
    allow_before_comment: bool,
    line_idx: usize,
    offset: usize,
    line: &[u8],
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let mut i = skip_indent(line);
    while i < line.len() {
        if line[i] != b' ' {
            i += 1;
            continue;
        }
        let start = i;
        while i < line.len() && line[i] == b' ' {
            i += 1;
        }
        check_run(
            cop,
            source,
            code_map,
            lines,
            ignored,
            allow_aligned,
            allow_before_comment,
            line_idx,
            offset,
            line,
            start,
            i,
            diagnostics,
            corrections,
        );
    }
}

impl Cop for ExtraSpacing {
    fn name(&self) -> &'static str {
        "Layout/ExtraSpacing"
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
        code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let allow_aligned = config.get_bool("AllowForAlignment", true);
        let allow_before_comment = config.get_bool("AllowBeforeTrailingComments", false);
        let mut ignored = Vec::new();
        collect_hash_gaps(tree.root_node(), &mut ignored);
        let lines: Vec<&[u8]> = source.lines().collect();
        let mut offset = 0usize;
        for (line_idx, line) in lines.iter().enumerate() {
            scan_line(
                self,
                source,
                code_map,
                &lines,
                &ignored,
                allow_aligned,
                allow_before_comment,
                line_idx,
                offset,
                line,
                diagnostics,
                &mut corrections,
            );
            offset += line.len() + 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ExtraSpacing, "cops/layout/extra_spacing");
}
