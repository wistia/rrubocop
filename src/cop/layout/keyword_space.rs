//! Helpers for Layout/SpaceAroundKeyword.

use tree_sitter::Node;

use crate::cop::Cop;
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub const KEYWORDS: &[&[u8]] = &[
    b"and",
    b"or",
    b"not",
    b"if",
    b"unless",
    b"while",
    b"until",
    b"for",
    b"in",
    b"do",
    b"then",
    b"when",
    b"else",
    b"elsif",
    b"begin",
    b"rescue",
    b"ensure",
    b"end",
    b"case",
    b"next",
    b"break",
    b"return",
    b"super",
    b"yield",
    b"defined?",
];

fn is_kw_kind(k: &str) -> bool {
    KEYWORDS.iter().any(|&kw| kw == k.as_bytes())
        || matches!(k, "return" | "break" | "next" | "yield" | "super")
}

fn is_anon_kw(bytes: &[u8], n: Node<'_>) -> bool {
    let t = &bytes[n.start_byte()..n.end_byte()];
    !n.is_named() && KEYWORDS.iter().any(|&kw| t == kw)
}

pub fn should_check(bytes: &[u8], n: Node<'_>) -> bool {
    // Prefer anonymous keyword tokens so container nodes (e.g. `if`) are not
    // double-reported alongside their `if` child.
    if is_anon_kw(bytes, n) {
        return true;
    }
    // Named leaves whose span is exactly the keyword (`return`, `super`, …).
    n.is_named()
        && leaf_kw_span(bytes, n).is_some_and(|(s, e)| e - s == n.end_byte() - n.start_byte())
}

fn do_end(n: Node<'_>) -> Option<Node<'_>> {
    let mut cur = n.walk();
    n.children(&mut cur).find(|c| c.kind() == "do")
}

fn keyword_token_span(bytes: &[u8], n: Node<'_>) -> Option<(usize, usize)> {
    let mut cur = n.walk();
    n.children(&mut cur).find_map(|c| {
        let t = &bytes[c.start_byte()..c.end_byte()];
        KEYWORDS
            .iter()
            .any(|&kw| t == kw)
            .then_some((c.start_byte(), c.end_byte()))
    })
}

fn leaf_kw_span(bytes: &[u8], n: Node<'_>) -> Option<(usize, usize)> {
    let k = n.kind();
    if !is_kw_kind(k) {
        return None;
    }
    let start = n.start_byte();
    let end = start + k.len();
    (end <= n.end_byte() && &bytes[start..end] == k.as_bytes()).then_some((start, end))
}

fn anon_kw_span(bytes: &[u8], n: Node<'_>) -> Option<(usize, usize)> {
    let t = &bytes[n.start_byte()..n.end_byte()];
    KEYWORDS
        .iter()
        .any(|&kw| t == kw)
        .then_some((n.start_byte(), n.end_byte()))
}

pub fn kw_span(bytes: &[u8], n: Node<'_>) -> Option<(usize, usize)> {
    if n.kind() == "do_block" {
        return do_end(n).map(|d| (d.start_byte(), d.end_byte()));
    }
    if !n.is_named() {
        return anon_kw_span(bytes, n);
    }
    keyword_token_span(bytes, n).or_else(|| leaf_kw_span(bytes, n))
}

fn need_space_after(next: u8) -> bool {
    next.is_ascii_alphanumeric()
        || matches!(next, b'_' | b'(' | b':' | b'"' | b'\'' | b'@' | b'$' | b'[')
}

fn skip_after_punct(next: u8) -> bool {
    next.is_ascii_whitespace() || matches!(next, b';' | b',' | b')' | b']' | b'}')
}

const ACCEPT_LEFT_PAREN: &[&str] = &[
    "break", "defined?", "next", "not", "rescue", "super", "yield", "while", "until",
];
const ACCEPT_LEFT_BRACKET: &[&str] = &["super", "yield"];

fn accept_punct(next: u8) -> bool {
    // RuboCop: /[\s;,#\\)}\].]/ plus `.` (already partly in skip_after_punct).
    skip_after_punct(next) || matches!(next, b'.' | b'\\' | b'#')
}

fn accept_kw_delim(k: &str, next: u8) -> bool {
    if k == "end" && next == b'[' {
        return true;
    }
    (next == b'[' && ACCEPT_LEFT_BRACKET.contains(&k))
        || (next == b'(' && ACCEPT_LEFT_PAREN.contains(&k))
}

fn accept_no_space(bytes: &[u8], k: &str, end: usize, next: u8) -> bool {
    if accept_punct(next) || accept_kw_delim(k, next) {
        return true;
    }
    match next {
        b':' => bytes.get(end + 1) == Some(&b':'),
        b'&' => bytes.get(end + 1) == Some(&b'.'),
        _ => false,
    }
}

fn missing_after(bytes: &[u8], k: &str, end: usize) -> bool {
    let Some(&next) = bytes.get(end) else {
        return false;
    };
    !accept_no_space(bytes, k, end, next) && (need_space_after(next) || next.is_ascii_graphic())
}

fn missing_before(bytes: &[u8], kw_start: usize) -> bool {
    if kw_start == 0 {
        return false;
    }
    let prev = bytes[kw_start - 1];
    if prev.is_ascii_whitespace() || matches!(prev, b'(' | b'{' | b'[' | b';' | b'!') {
        return false;
    }
    prev.is_ascii_alphanumeric() || matches!(prev, b'_' | b')' | b']' | b'}' | b'?' | b'!')
}

fn report_space(
    cop: &dyn Cop,
    source: &SourceFile,
    at: usize,
    insert_at: usize,
    msg: String,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (l, c) = source.offset_to_line_col(at);
    let mut diag = cop.diagnostic(source, l, c, msg);
    if let Some(corr) = corrections {
        corr.push(Correction {
            start: insert_at,
            end: insert_at,
            replacement: " ".into(),
            cop_name: cop.name(),
            cop_index: 0,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

pub fn check_after(
    cop: &dyn Cop,
    source: &SourceFile,
    bytes: &[u8],
    k: &str,
    kw_start: usize,
    end: usize,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    if !missing_after(bytes, k, end) {
        return;
    }
    let kw = String::from_utf8_lossy(&bytes[kw_start..end]);
    report_space(
        cop,
        source,
        kw_start,
        end,
        format!("Space after keyword `{kw}` is missing."),
        diagnostics,
        corrections,
    );
}

pub fn check_before(
    cop: &dyn Cop,
    source: &SourceFile,
    bytes: &[u8],
    kw_start: usize,
    end: usize,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    if !missing_before(bytes, kw_start) {
        return;
    }
    let kw = String::from_utf8_lossy(&bytes[kw_start..end]);
    report_space(
        cop,
        source,
        kw_start,
        kw_start,
        format!("Space before keyword `{kw}` is missing."),
        diagnostics,
        corrections,
    );
}
