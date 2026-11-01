//! Parse `# rubocop:disable` directives and locate cop names in comment text.

use std::collections::HashMap;

use crate::parse::source::{SourceFile, byte_index_to_column};

#[derive(Clone, Debug)]
pub(super) struct DisableDirective {
    pub line: usize,
    pub column: usize,
    pub cops: Vec<String>,
    pub range: (usize, usize),
}

pub(super) fn disable_marker(line: &str) -> Option<(usize, &str)> {
    let lower = line.to_ascii_lowercase();
    let marker = "# rubocop:disable";
    let pos = lower.find(marker)?;
    Some((
        byte_index_to_column(line, pos),
        line[pos + marker.len()..].trim(),
    ))
}

fn enable_marker(line: &str) -> Option<&str> {
    let lower = line.to_ascii_lowercase();
    let marker = "# rubocop:enable";
    let pos = lower.find(marker)?;
    Some(line[pos + marker.len()..].trim())
}

pub(super) fn cop_names(rest: &str) -> Vec<String> {
    let rest = rest.trim_start_matches(':').trim();
    rest.split("--")
        .next()
        .unwrap_or("")
        .split(',')
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty())
        .collect()
}

fn open_block_key(open: &HashMap<String, DisableDirective>, name: String) -> String {
    open.keys()
        .find(|k| k.eq_ignore_ascii_case(&name))
        .cloned()
        .unwrap_or(name)
}

fn apply_enable(
    open: &mut HashMap<String, DisableDirective>,
    out: &mut Vec<DisableDirective>,
    line_no: usize,
    rest: &str,
) {
    let names = cop_names(rest);
    if names.iter().any(|n| n.eq_ignore_ascii_case("all")) {
        for (_, mut dir) in open.drain() {
            dir.range = (dir.range.0, line_no);
            out.push(dir);
        }
        return;
    }
    for name in names {
        if let Some(mut dir) = open.remove(&open_block_key(open, name)) {
            dir.range = (dir.range.0, line_no);
            out.push(dir);
        }
    }
}

fn record_disable(
    open: &mut HashMap<String, DisableDirective>,
    out: &mut Vec<DisableDirective>,
    line_no: usize,
    col: usize,
    line: &str,
    cops: Vec<String>,
) {
    // Bare `# rubocop:disable` (no cop names) is Lint/CopDirectiveSyntax, not "all".
    if cops.is_empty() {
        return;
    }
    if line.trim_start().starts_with('#') {
        for name in cops {
            open.insert(
                name.clone(),
                DisableDirective {
                    line: line_no,
                    column: col,
                    cops: vec![name],
                    range: (line_no, line_no),
                },
            );
        }
    } else {
        out.push(DisableDirective {
            line: line_no,
            column: col,
            cops,
            range: (line_no, line_no),
        });
    }
}

fn flush_open_directives(
    open: HashMap<String, DisableDirective>,
    end_line: usize,
    out: &mut Vec<DisableDirective>,
) {
    for (_, mut dir) in open {
        dir.range = (dir.range.0, end_line);
        out.push(dir);
    }
}

pub(super) fn collect_directives(source: &SourceFile) -> Vec<DisableDirective> {
    let mut out = Vec::new();
    let mut open = HashMap::new();
    let directives = crate::parse::directives::cached(source);
    for &line_no in directives.comment_lines() {
        let Some(s) = directives.comment_line(source, line_no) else {
            continue;
        };
        if let Some(rest) = enable_marker(&s) {
            apply_enable(&mut open, &mut out, line_no, rest);
        }
        if let Some((col, rest)) = disable_marker(&s) {
            record_disable(&mut open, &mut out, line_no, col, &s, cop_names(rest));
        }
    }
    flush_open_directives(open, source.line_count().max(1), &mut out);
    out
}

fn whole_token(line: &str, byte: usize, len: usize) -> bool {
    let before = byte == 0 || !line.as_bytes()[byte - 1].is_ascii_alphanumeric();
    let after_byte = byte + len;
    before
        && line
            .get(after_byte..)
            .map(|rest| {
                rest.is_empty()
                    || !rest.as_bytes()[0].is_ascii_alphanumeric() && rest.as_bytes()[0] != b'_'
            })
            .unwrap_or(true)
}

pub(crate) fn nth_cop_token(line: &str, cop: &str, occurrence: usize) -> Option<(usize, usize)> {
    let lower = line.to_ascii_lowercase();
    let needle = cop.to_ascii_lowercase();
    let mut found = 0usize;
    let mut search = 0usize;
    while let Some(pos) = lower[search..].find(&needle) {
        let byte = search + pos;
        if whole_token(line, byte, needle.len()) {
            found += 1;
            if found == occurrence {
                return Some((byte, byte + needle.len()));
            }
        }
        search = byte + 1;
    }
    None
}

pub(super) fn cop_token_column(line: &str, cop: &str, occurrence: usize) -> usize {
    nth_cop_token(line, cop, occurrence)
        .map(|(byte, _)| byte_index_to_column(line, byte))
        .unwrap_or(0)
}

pub(super) fn offense_span(line: &str, dir: &DisableDirective, name: &str) -> (usize, usize) {
    if dir.cops.len() == 1
        && let Some((_, end)) = nth_cop_token(line, name, 1)
    {
        return (
            dir.column,
            byte_index_to_column(line, end).saturating_sub(dir.column),
        );
    }
    let column = redundant_col(line, name, dir.column);
    (column, cop_highlight(line, column, name))
}

pub(super) fn redundant_col(line: &str, name: &str, fallback: usize) -> usize {
    nth_cop_token(line, name, 1)
        .map(|(byte, _)| byte_index_to_column(line, byte))
        .unwrap_or(fallback)
}

pub(super) fn cop_highlight(line: &str, col: usize, cop: &str) -> usize {
    line.char_indices()
        .nth(col)
        .map(|(i, _)| &line[i..])
        .unwrap_or("")
        .split(',')
        .next()
        .unwrap_or("")
        .trim()
        .find(cop)
        .map(|_| cop.len())
        .unwrap_or(1)
}
