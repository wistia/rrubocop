pub(super) fn singleton(inner: &str) -> bool {
    if inner.chars().count() == 1 {
        return true;
    }
    if !inner.starts_with('\\') {
        return false;
    }
    let bytes = inner.as_bytes();
    if bytes.len() == 2 {
        return true;
    }
    match bytes[1] {
        b'x' => hex(inner, 4),
        b'u' => unicode(inner),
        b'p' | b'P' => property(inner),
        b'c' | b'C' | b'M' => super::escape_token::end(inner, 0) == inner.len(),
        b'0'..=b'7' => octal(inner),
        _ => false,
    }
}
fn hex(inner: &str, len: usize) -> bool {
    inner.len() == len && inner[2..].chars().all(|c| c.is_ascii_hexdigit())
}
fn unicode(inner: &str) -> bool {
    if inner.as_bytes().get(2) == Some(&b'{') {
        braced_codepoint(inner)
    } else {
        hex(inner, 6)
    }
}
fn property(inner: &str) -> bool {
    inner.get(2..3) == Some("{") && inner.ends_with('}') && !inner[3..inner.len() - 1].contains('}')
}

fn octal(inner: &str) -> bool {
    inner.len() <= 4 && inner[1..].chars().all(|c| ('0'..='7').contains(&c))
}

fn braced_codepoint(inner: &str) -> bool {
    inner
        .strip_prefix("\\u{")
        .and_then(|value| value.strip_suffix('}'))
        .is_some_and(valid_codepoint)
}
fn valid_codepoint(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value.len() <= 6
        && value.chars().all(|ch| ch.is_ascii_hexdigit())
        && u32::from_str_radix(value, 16)
            .ok()
            .and_then(char::from_u32)
            .is_some()
}
