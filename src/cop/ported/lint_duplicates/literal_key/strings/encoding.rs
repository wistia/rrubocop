use super::*;
use std::sync::{Arc, OnceLock};
enum SourceEncoding {
    Utf8,
    Binary,
    Other,
}

pub(super) fn supported_raw(c: &Context<'_>, n: Node<'_>) -> bool {
    c.source.as_bytes()[n.byte_range()].is_ascii() || !matches!(*source(c), SourceEncoding::Other)
}
pub(super) fn name(c: &Context<'_>, bytes: &[u8], unicode: bool) -> &'static str {
    if bytes.is_ascii() {
        "ASCII"
    } else if unicode || !matches!(*source(c), SourceEncoding::Binary) {
        "UTF-8"
    } else {
        "ASCII-8BIT"
    }
}
fn source(c: &Context<'_>) -> Arc<SourceEncoding> {
    c.source.cached(0, || source_encoding(c))
}
fn source_encoding(c: &Context<'_>) -> SourceEncoding {
    let name = header(c).and_then(encoding).map(str::to_ascii_lowercase);
    match name.as_deref() {
        Some("binary" | "ascii-8bit") => SourceEncoding::Binary,
        Some("utf8" | "utf-8") | None => SourceEncoding::Utf8,
        _ => SourceEncoding::Other,
    }
}
fn header<'a>(c: &Context<'a>) -> Option<&'a str> {
    let mut lines = c.source.as_bytes().split(|byte| *byte == b'\n');
    let first = lines.next()?;
    let first = first.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(first);
    let line = if first.starts_with(b"#!") {
        lines.next()?
    } else {
        first
    };
    std::str::from_utf8(line).ok()
}
fn encoding(line: &str) -> Option<&str> {
    static PATTERN: OnceLock<regex::Regex> = OnceLock::new();
    let line = line.trim_start().strip_prefix('#')?;
    PATTERN
        .get_or_init(|| regex::Regex::new(r"\b(?:en)?coding\s*[:=]\s*([\w-]+)").unwrap())
        .captures(line)?
        .get(1)
        .map(|matched| matched.as_str())
}
