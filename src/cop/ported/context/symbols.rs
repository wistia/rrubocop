use super::*;
use std::borrow::Cow;

pub(super) fn value<'a>(c: &Context<'a>, n: Node<'_>) -> Option<Cow<'a, str>> {
    match n.kind() {
        "simple_symbol" => Some(Cow::Borrowed(c.text(n).strip_prefix(':')?)),
        "hash_key_symbol" => Some(Cow::Borrowed(c.text(n))),
        "delimited_symbol" => delimited(c, n),
        "string" if symbol_label(c, n) => delimited(c, n),
        _ => None,
    }
}
fn symbol_label(c: &Context<'_>, n: Node<'_>) -> bool {
    c.source.as_bytes().get(n.end_byte()) == Some(&b':')
        && n.parent()
            .is_some_and(|p| p.kind() == "pair" && field(p, "key") == Some(n))
}
pub(super) fn string<'a>(c: &Context<'a>, n: Node<'_>) -> Option<Cow<'a, str>> {
    match n.kind() {
        "string" if !symbol_label(c, n) => delimited(c, n),
        "chained_string" => chained(c, n),
        _ => None,
    }
}
fn chained<'a>(c: &Context<'a>, n: Node<'_>) -> Option<Cow<'a, str>> {
    let mut cursor = n.walk();
    let mut result = String::new();
    for child in n
        .named_children(&mut cursor)
        .filter(|child| child.kind() != "comment")
    {
        result.push_str(string(c, child)?.as_ref());
    }
    Some(Cow::Owned(result))
}
fn delimited<'a>(c: &Context<'a>, n: Node<'_>) -> Option<Cow<'a, str>> {
    if c.text(n).contains('#') && descendants(n).any(|n| n.kind() == "interpolation") {
        return None;
    }
    let (inner, double, delimiter) = contents(c.text(n))?;
    if !inner.contains('\\') {
        return Some(Cow::Borrowed(inner));
    }
    decode(inner, double, delimiter).map(Cow::Owned)
}
fn contents(text: &str) -> Option<(&str, bool, u8)> {
    let text = text.strip_prefix(':').unwrap_or(text);
    let (offset, double) = match text.as_bytes() {
        [b'%', b's' | b'q', ..] => (2, false),
        [b'%', b'Q', ..] => (2, true),
        [b'%', ..] => (1, true),
        [b'"', ..] => (0, true),
        [b'\'', ..] => (0, false),
        _ => return None,
    };
    let delimiter = *text.as_bytes().get(offset)?;
    let inner = text.get(offset + 1..text.len().checked_sub(1)?)?;
    Some((inner, double, delimiter))
}
fn decode(inner: &str, double: bool, delimiter: u8) -> Option<String> {
    let mut bytes = inner.bytes().peekable();
    let mut result = Vec::with_capacity(inner.len());
    while let Some(byte) = bytes.next() {
        if byte != b'\\' {
            result.push(byte);
        } else if double {
            escapes::append(&mut bytes, &mut result)?;
        } else {
            single_escape(&mut bytes, &mut result, delimiter)?;
        }
    }
    String::from_utf8(result).ok()
}
fn closing(open: u8) -> u8 {
    match open {
        b'(' => b')',
        b'[' => b']',
        b'{' => b'}',
        b'<' => b'>',
        _ => open,
    }
}
mod escapes;
mod ruby;
pub(super) use ruby::string as ruby_string;
pub(super) use ruby::{heredoc_span, parts as ruby_parts};
mod duplicate;
pub(super) use duplicate::value as duplicate_bytes;

fn continued(line: &str) -> bool {
    line.strip_suffix('\n')
        .is_some_and(|line| line.bytes().rev().take_while(|byte| *byte == b'\\').count() % 2 == 1)
}
fn decoded_parts(content: &str, mode: Option<(bool, u8)>) -> Option<Vec<String>> {
    let mut parts = Vec::new();
    let mut group = String::new();
    for line in content.split_inclusive('\n') {
        group.push_str(line);
        if !mode.is_some_and(|(double, _)| double) || !continued(line) {
            parts.push(match mode {
                Some((double, delimiter)) if group.contains('\\') => {
                    decode(&group, double, delimiter)?
                }
                _ => std::mem::take(&mut group),
            });
            group.clear();
        }
    }
    if !group.is_empty() {
        parts.push(match mode {
            Some((double, delimiter)) if group.contains('\\') => decode(&group, double, delimiter)?,
            _ => group,
        });
    }
    Some(parts)
}
fn string_parts(c: &Context<'_>, n: Node<'_>) -> Option<Vec<String>> {
    if descendants(n).any(|n| n.kind() == "interpolation") {
        return None;
    }
    let (content, double, delimiter) = contents(c.text(n))?;
    let content = content.replace("\r\n", "\n");
    let mut parts = decoded_parts(&content, Some((double, delimiter)))?;
    if parts.is_empty() {
        parts.push(String::new());
    }
    Some(parts)
}

fn single_escape(
    bytes: &mut std::iter::Peekable<std::str::Bytes<'_>>,
    result: &mut Vec<u8>,
    delimiter: u8,
) -> Option<()> {
    let escaped = bytes.next()?;
    if escaped != b'\\' && escaped != delimiter && escaped != closing(delimiter) {
        result.push(b'\\');
    }
    result.push(escaped);
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn double_quoted_space_escape_differs_from_single_quoted() {
        assert_eq!(decode(r"\s", true, b'"').as_deref(), Some(" "));
        assert_eq!(decode(r"\s", false, b'\'').as_deref(), Some(r"\s"));
    }
    #[test]
    fn nested_control_escapes_do_not_recurse() {
        let text = format!("{}A", r"\C-".repeat(10_000));
        assert_eq!(decode(&text, true, b'"').as_deref(), Some("\x01"));
    }
}

pub(super) fn heredoc_end(c: &Context<'_>, n: Node<'_>) -> Option<usize> {
    ruby::heredoc_end(c, n)
}

pub(in crate::cop::ported) fn decode_double_quoted_content(inner: &str) -> Option<String> {
    decode(inner, true, b'"')
}

pub(in crate::cop::ported) fn decode_regexp_content(
    inner: &[u8],
    opening: u8,
    closing: u8,
    ruby_version: f64,
) -> Option<Vec<u8>> {
    let text = std::str::from_utf8(inner).ok()?;
    let mut bytes = text.bytes().peekable();
    let mut output = Vec::with_capacity(inner.len());
    while let Some(byte) = bytes.next() {
        if byte != b'\\' {
            output.push(byte);
            continue;
        }
        let escaped = *bytes.peek()?;
        if (escaped == opening || escaped == closing) && !b"\\$()*+.<>?[]^{|}".contains(&escaped) {
            output.push(bytes.next()?);
        } else if escaped == b'\n'
            || escaped == b'\r' && bytes.clone().nth(1) == Some(b'\n')
            || ruby_version >= 3.1 && matches!(escaped, b'c' | b'C' | b'm' | b'M')
        {
            escapes::append(&mut bytes, &mut output)?;
        } else {
            output.push(byte);
            output.push(bytes.next()?);
        }
    }
    Some(output)
}
