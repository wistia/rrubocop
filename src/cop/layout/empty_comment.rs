//! Layout/EmptyComment.

use tree_sitter::{Node, Tree};

use crate::cop::shared;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct EmptyComment;

fn is_empty_comment(text: &[u8]) -> bool {
    text.starts_with(b"#")
        && !text[1..]
            .iter()
            .any(|&b| b != b' ' && b != b'\t' && b != b'\r')
}

fn is_border(text: &[u8]) -> bool {
    text.len() >= 2 && text.iter().all(|&b| b == b'#')
}

fn line_start_of(bytes: &[u8], start: usize) -> usize {
    bytes[..start]
        .iter()
        .rposition(|&b| b == b'\n')
        .map(|i| i + 1)
        .unwrap_or(0)
}

fn comment_column(bytes: &[u8], start: usize) -> usize {
    start - line_start_of(bytes, start)
}

fn removal_range(bytes: &[u8], start: usize, end: usize) -> (usize, usize) {
    let line_start = line_start_of(bytes, start);
    let before = &bytes[line_start..start];
    if !before.iter().all(|&b| b == b' ' || b == b'\t') {
        let mut s = start;
        while s > line_start && matches!(bytes[s - 1], b' ' | b'\t') {
            s -= 1;
        }
        return (s, end);
    }
    let mut e = end;
    if e < bytes.len() && bytes[e] == b'\n' {
        e += 1;
    }
    (line_start, e)
}

fn report_empty(
    cop: &dyn Cop,
    source: &SourceFile,
    bytes: &[u8],
    start: usize,
    end: usize,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (line, col) = source.offset_to_line_col(start);
    let mut diag = cop.diagnostic(source, line, col, "Source code comment is empty.".into());
    if let Some(corr) = corrections {
        let (s, e) = removal_range(bytes, start, end);
        corr.push(Correction {
            start: s,
            end: e,
            replacement: String::new(),
            cop_name: cop.name(),
            cop_index: 0,
            owner: None,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

fn empty_chunk_only(joined: &str, allow_border: bool) -> bool {
    let ok = |line: &str| {
        if allow_border {
            line == "#"
        } else {
            !line.is_empty() && line.bytes().all(|b| b == b'#')
        }
    };
    !joined.is_empty() && joined.ends_with('\n') && joined.lines().all(ok)
}

fn should_flag_alone(text: &[u8], allow_border: bool) -> bool {
    is_empty_comment(text) || (!allow_border && is_border(text))
}

fn scan_alone(
    cop: &dyn Cop,
    source: &SourceFile,
    bytes: &[u8],
    comments: &[Node<'_>],
    allow_border: bool,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    for c in comments {
        let (s, e) = (c.start_byte(), c.end_byte());
        if should_flag_alone(&bytes[s..e], allow_border) {
            report_empty(cop, source, bytes, s, e, diagnostics, corrections);
        }
    }
}

fn chunk_end(source: &SourceFile, bytes: &[u8], comments: &[Node<'_>], start: usize) -> usize {
    let mut j = start + 1;
    while j < comments.len() {
        let prev = comments[j - 1];
        let cur = comments[j];
        let same_col =
            comment_column(bytes, prev.start_byte()) == comment_column(bytes, cur.start_byte());
        let consecutive = shared::node_line(source, prev) + 1 == shared::node_line(source, cur);
        if !(same_col && consecutive) {
            break;
        }
        j += 1;
    }
    j
}

fn join_chunk(bytes: &[u8], chunk: &[Node<'_>]) -> String {
    let mut joined = String::new();
    for c in chunk {
        joined.push_str(
            std::str::from_utf8(&bytes[c.start_byte()..c.end_byte()])
                .unwrap_or("")
                .trim(),
        );
        joined.push('\n');
    }
    joined
}

fn scan_margin(
    cop: &dyn Cop,
    source: &SourceFile,
    bytes: &[u8],
    comments: &[Node<'_>],
    allow_border: bool,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let mut i = 0;
    while i < comments.len() {
        let j = chunk_end(source, bytes, comments, i);
        let chunk = &comments[i..j];
        if empty_chunk_only(&join_chunk(bytes, chunk), allow_border) {
            for c in chunk {
                report_empty(
                    cop,
                    source,
                    bytes,
                    c.start_byte(),
                    c.end_byte(),
                    diagnostics,
                    corrections,
                );
            }
        }
        i = j;
    }
}

impl Cop for EmptyComment {
    fn name(&self) -> &'static str {
        "Layout/EmptyComment"
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
        let allow_border = config.get_bool("AllowBorderComment", true);
        let bytes = source.as_bytes();
        let comments = shared::collect_comments(tree.root_node());
        if config.get_bool("AllowMarginComment", true) {
            scan_margin(
                self,
                source,
                bytes,
                &comments,
                allow_border,
                diagnostics,
                &mut corrections,
            );
        } else {
            scan_alone(
                self,
                source,
                bytes,
                &comments,
                allow_border,
                diagnostics,
                &mut corrections,
            );
        }
    }
}
