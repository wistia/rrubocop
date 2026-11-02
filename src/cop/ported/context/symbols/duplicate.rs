use super::*;

pub(in crate::cop::ported::context) fn value(
    c: &Context<'_>,
    n: Node<'_>,
) -> Option<(Vec<u8>, bool)> {
    let text = std::str::from_utf8(c.source.as_bytes().get(n.byte_range())?).ok()?;
    if text.contains('#') && descendants(n).any(|n| n.kind() == "interpolation") {
        return None;
    }
    unquoted(n.kind(), text)
}
fn unquoted(kind: &str, text: &str) -> Option<(Vec<u8>, bool)> {
    match kind {
        "simple_symbol" => Some((text.strip_prefix(':')?.as_bytes().to_vec(), false)),
        "hash_key_symbol" => Some((text.as_bytes().to_vec(), false)),
        "bare_string" | "bare_symbol" if !text.contains('\\') => {
            Some((text.as_bytes().to_vec(), false))
        }
        _ => quoted(kind, text),
    }
}
fn quoted(kind: &str, text: &str) -> Option<(Vec<u8>, bool)> {
    match kind {
        "character" => decode(text.strip_prefix('?')?, true, b'"'),
        "string" | "delimited_symbol" => {
            let (inner, double, delimiter) = contents(text)?;
            decode(inner, double, delimiter)
        }
        _ => None,
    }
}
fn decode(inner: &str, double: bool, delimiter: u8) -> Option<(Vec<u8>, bool)> {
    if !inner.contains('\\') {
        return Some((inner.as_bytes().to_vec(), false));
    }
    decode_escaped(inner, double, delimiter)
}
fn decode_escaped(inner: &str, double: bool, delimiter: u8) -> Option<(Vec<u8>, bool)> {
    let mut bytes = inner.bytes().peekable();
    let mut result = Vec::with_capacity(inner.len());
    let mut unicode = false;
    while let Some(byte) = bytes.next() {
        if byte != b'\\' {
            result.push(byte);
        } else if double {
            unicode |= escaped(&mut bytes, &mut result)?;
        } else {
            single_escape(&mut bytes, &mut result, delimiter)?;
        }
    }
    Some((result, unicode))
}
fn escaped(
    bytes: &mut std::iter::Peekable<std::str::Bytes<'_>>,
    result: &mut Vec<u8>,
) -> Option<bool> {
    let unicode = bytes.peek() == Some(&b'u');
    let start = result.len();
    escapes::append(bytes, result)?;
    Some(unicode && !result[start..].is_ascii())
}
