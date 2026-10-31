//! Group flush, sort keys, and gem-name parsing for Gemspec/OrderedDependencies.

use crate::cop::Cop;
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

use super::ordered_dependencies::OrderedDependencies;

pub(super) const DEP_METHODS: &[&str] = &[
    "add_dependency",
    "add_runtime_dependency",
    "add_development_dependency",
];

pub(super) struct DepEntry {
    pub gem_name: String,
    pub sort_key: String,
    pub line_num: usize,
    pub col: usize,
    pub line_start: usize,
    pub line_end: usize,
}

/// Byte ranges of each source line (start inclusive, end exclusive, includes newline).
pub(super) fn line_offsets(source: &SourceFile) -> Vec<(usize, usize)> {
    let bytes = source.as_bytes();
    let mut offset = 0;
    source
        .lines()
        .map(|line| {
            let start = offset;
            offset += line.len();
            if offset < bytes.len() && bytes[offset] == b'\n' {
                offset += 1;
            }
            (start, offset)
        })
        .collect()
}

pub(super) fn sort_key(name: &str, consider_punctuation: bool) -> String {
    if consider_punctuation {
        return name.to_lowercase();
    }
    name.chars()
        .filter(|&c| c != '-' && c != '_')
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

pub(super) fn flush_group(
    group: &mut Vec<DepEntry>,
    diagnostics: &mut Vec<Diagnostic>,
    source: &SourceFile,
    cop: &OrderedDependencies,
    corrections: &mut Option<&mut Vec<Correction>>,
    bytes: &[u8],
) {
    if group.len() < 2 {
        group.clear();
        return;
    }
    report_unsorted(group, diagnostics, source, cop, corrections.is_some());
    if let Some(corr) = corrections {
        push_sorted_correction(group, corr, bytes);
    }
    group.clear();
}

fn report_unsorted(
    group: &[DepEntry],
    diagnostics: &mut Vec<Diagnostic>,
    source: &SourceFile,
    cop: &OrderedDependencies,
    will_correct: bool,
) {
    for i in 1..group.len() {
        if group[i].sort_key >= group[i - 1].sort_key {
            continue;
        }
        let mut diag = cop.diagnostic(
            source,
            group[i].line_num,
            group[i].col,
            format!(
                "Dependencies should be sorted in an alphabetical order within their section of the gemspec. Dependency `{}` should appear before `{}`.",
                group[i].gem_name, group[i - 1].gem_name
            ),
        );
        diag.corrected = will_correct;
        diagnostics.push(diag);
    }
}

fn push_sorted_correction(group: &[DepEntry], corr: &mut Vec<Correction>, bytes: &[u8]) {
    if group.windows(2).all(|w| w[0].sort_key <= w[1].sort_key) {
        return;
    }
    let mut indices: Vec<usize> = (0..group.len()).collect();
    indices.sort_by_key(|&i| group[i].sort_key.as_str());
    corr.push(Correction {
        start: group[0].line_start,
        end: group[group.len() - 1].line_end,
        replacement: indices
            .into_iter()
            .map(|i| {
                String::from_utf8_lossy(&bytes[group[i].line_start..group[i].line_end]).into_owned()
            })
            .collect(),
        cop_name: "Gemspec/OrderedDependencies",
        cop_index: 0,
        owner: None,
    });
}

/// Parse a dependency call on `line_str`; push into `group` when found.
pub(super) fn try_add_dep(
    line_str: &str,
    line_idx: usize,
    line_start: usize,
    line_end: usize,
    current_method: &mut Option<String>,
    group: &mut Vec<DepEntry>,
    mut flush: impl FnMut(&mut Vec<DepEntry>),
    consider_punctuation: bool,
) -> bool {
    let Some((method, pos, gem_name)) = find_dep(line_str) else {
        return false;
    };
    if current_method.as_deref() != Some(method) {
        flush(group);
        *current_method = Some(method.to_string());
    }
    group.push(DepEntry {
        sort_key: sort_key(&gem_name, consider_punctuation),
        gem_name,
        line_num: line_idx + 1,
        col: pos + 1,
        line_start,
        line_end,
    });
    true
}

fn find_dep(line_str: &str) -> Option<(&'static str, usize, String)> {
    for &method in DEP_METHODS {
        let needle = format!(".{method}");
        let Some(pos) = line_str.find(&needle) else {
            continue;
        };
        let after = &line_str[pos + needle.len()..];
        return Some((method, pos, extract_gem_name(after)?));
    }
    None
}

/// Gem name after a dependency method call; `None` when argument uses `.freeze`.
pub(super) fn extract_gem_name(after_method: &str) -> Option<String> {
    let s = after_method.trim_start();
    let s = s.strip_prefix('(').map_or(s, str::trim_start);
    if s.starts_with('\'') || s.starts_with('"') {
        return quoted_gem_name(s);
    }
    percent_gem_name(s)
}

fn quoted_gem_name(s: &str) -> Option<String> {
    let quote = s.as_bytes()[0];
    let rest = &s[1..];
    let end = rest.find(|c: char| c as u8 == quote)?;
    (!rest[end + 1..].trim_start().starts_with(".freeze")).then(|| rest[..end].to_string())
}

fn percent_gem_name(s: &str) -> Option<String> {
    let (name, consumed) = parse_percent_string(s)?;
    (!s[consumed..].trim_start().starts_with(".freeze")).then_some(name)
}

/// Parse `%q<...>`, `%Q(...)`, etc. → (name, bytes consumed).
fn parse_percent_string(s: &str) -> Option<(String, usize)> {
    let rest = s.strip_prefix('%')?;
    let (body, base) = strip_q(rest);
    let (close, open_len) = percent_delim(body)?;
    let inner = &body[open_len..];
    let end = inner.find(close)?;
    Some((inner[..end].to_string(), base + open_len + end + 1))
}

fn strip_q(rest: &str) -> (&str, usize) {
    match rest.as_bytes().first() {
        Some(&b'q' | &b'Q') => (&rest[1..], 2),
        _ => (rest, 1),
    }
}

fn percent_delim(body: &str) -> Option<(char, usize)> {
    Some(match body.as_bytes().first()? {
        b'<' => ('>', 1),
        b'(' => (')', 1),
        b'[' => (']', 1),
        b'{' => ('}', 1),
        _ => return None,
    })
}
