//! Layout/TrailingWhitespace — adapted from nitrocop (line-based).

use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct TrailingWhitespace;

fn trailing_whitespace_start(line: &[u8]) -> Option<usize> {
    let mut end = line.len();
    let mut found = false;
    while end > 0 {
        if matches!(line[end - 1], b' ' | b'\t') {
            end -= 1;
            found = true;
            continue;
        }
        if end >= 3 && line[end - 3..end] == [0xE3, 0x80, 0x80] {
            end -= 3;
            found = true;
            continue;
        }
        if end >= 2 && line[end - 2..end] == [0xC2, 0xA0] {
            end -= 2;
            found = true;
            continue;
        }
        break;
    }
    found.then_some(end)
}

fn strip_cr(line: &[u8]) -> &[u8] {
    line.strip_suffix(b"\r").unwrap_or(line)
}

fn before_shift(s: &str, idx: usize) -> bool {
    if idx == 0 {
        return false;
    }
    let before = &s.as_bytes()[..idx];
    let Some(&b) = before.iter().rev().find(|&&b| b != b' ' && b != b'\t') else {
        return false;
    };
    matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b')' | b']' | b'}' | b'@' | b'$')
}

fn strip_heredoc_prefix(rest: &str) -> &str {
    rest.strip_prefix('~')
        .or_else(|| rest.strip_prefix('-'))
        .unwrap_or(rest)
        .trim_start()
}

fn take_ident(rest: &str) -> String {
    rest.chars()
        .skip_while(|c| *c == '\'' || *c == '"' || *c == '`')
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect()
}

fn parse_heredoc_ident(rest: &str) -> Option<Vec<u8>> {
    let rest = strip_heredoc_prefix(rest);
    if rest.is_empty() || rest.as_bytes()[0].is_ascii_digit() {
        return None;
    }
    let ident = take_ident(rest);
    if ident.is_empty() {
        None
    } else {
        Some(ident.into_bytes())
    }
}

fn heredoc_opener(line: &[u8]) -> Option<Vec<u8>> {
    let s = std::str::from_utf8(line).ok()?;
    let idx = s.find("<<")?;
    if before_shift(s, idx) {
        return None;
    }
    parse_heredoc_ident(&s[idx + 2..])
}

fn utf8_col(prefix: &[u8]) -> usize {
    prefix.iter().filter(|&&b| (b & 0xC0) != 0x80).count()
}

fn report_trailing(
    cop: &dyn Cop,
    source: &SourceFile,
    line_no: usize,
    start: usize,
    stripped: &[u8],
    byte_offset: usize,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let mut diag = cop.diagnostic(
        source,
        line_no,
        utf8_col(&stripped[..start]),
        "Trailing whitespace detected.".into(),
    );
    if let Some(corr) = corrections {
        corr.push(Correction {
            start: byte_offset + start,
            end: byte_offset + stripped.len(),
            replacement: String::new(),
            cop_name: cop.name(),
            cop_index: 0,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

fn in_heredoc_skip(stripped: &[u8], allow: bool, terms: &mut Vec<Vec<u8>>) -> bool {
    let Some(term) = terms.last() else {
        return false;
    };
    if stripped
        .iter()
        .copied()
        .skip_while(|&b| b == b' ' || b == b'\t')
        .collect::<Vec<u8>>()
        == *term
    {
        terms.pop();
        false
    } else {
        allow
    }
}

fn process_line(
    cop: &dyn Cop,
    source: &SourceFile,
    line_no: usize,
    line: &[u8],
    allow: bool,
    terms: &mut Vec<Vec<u8>>,
    saw_nonblank: &mut bool,
    byte_offset: usize,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) -> bool {
    let stripped = strip_cr(line);
    if in_heredoc_skip(stripped, allow, terms) {
        return true;
    }
    if stripped == b"__END__" && terms.is_empty() && *saw_nonblank {
        return false;
    }
    if !stripped.iter().all(|&b| b == b' ' || b == b'\t') {
        *saw_nonblank = true;
    }
    if let Some(start) = trailing_whitespace_start(stripped) {
        report_trailing(
            cop,
            source,
            line_no,
            start,
            stripped,
            byte_offset,
            diagnostics,
            corrections,
        );
    }
    if let Some(term) = heredoc_opener(stripped) {
        terms.push(term);
    }
    true
}

fn heredoc_ranges(tree: &tree_sitter::Tree) -> Vec<(usize, usize)> {
    let mut heredocs = Vec::new();
    crate::cop::shared::for_each_descendant(tree.root_node(), |n| {
        if n.kind() == "heredoc_body" {
            heredocs.push((n.start_byte(), n.end_byte()));
        }
    });
    heredocs
}

fn allowed_heredoc_line(allow: bool, ranges: &[(usize, usize)], offset: usize) -> bool {
    allow
        && ranges
            .iter()
            .any(|&(start, end)| offset >= start && offset < end)
}

impl Cop for TrailingWhitespace {
    fn name(&self) -> &'static str {
        "Layout/TrailingWhitespace"
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
        tree: &tree_sitter::Tree,
        _code_map: &crate::parse::codemap::CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let allow = config.get_bool("AllowInHeredoc", false);
        let heredocs = heredoc_ranges(tree);
        let mut terms = Vec::new();
        let mut saw_nonblank = false;
        let mut byte_offset = 0usize;
        for (i, line) in source.lines().enumerate() {
            if allowed_heredoc_line(allow, &heredocs, byte_offset) {
                byte_offset += line.len() + 1;
                continue;
            }
            if !process_line(
                self,
                source,
                i + 1,
                line,
                allow,
                &mut terms,
                &mut saw_nonblank,
                byte_offset,
                diagnostics,
                &mut corrections,
            ) {
                break;
            }
            byte_offset += line.len() + 1;
        }
    }
}
