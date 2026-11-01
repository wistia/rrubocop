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
fn delimited<'a>(c: &Context<'a>, n: Node<'_>) -> Option<Cow<'a, str>> {
    if descendants(n).any(|n| n.kind() == "interpolation") {
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
    let offset = if text.starts_with("%s") { 2 } else { 0 };
    let delimiter = *text.as_bytes().get(offset)?;
    let inner = text.get(offset + 1..text.len().checked_sub(1)?)?;
    Some((inner, offset == 0 && delimiter == b'"', delimiter))
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
}
