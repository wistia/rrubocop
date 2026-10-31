pub(super) fn end(text: &str, start: usize) -> usize {
    let value = &text[start + 1..];
    let Some(ch) = value.chars().next() else {
        return text.len();
    };
    let initial = start + 1 + ch.len_utf8();
    let delimiter = match ch {
        'p' | 'P' | 'u' if value.as_bytes().get(1) == Some(&b'{') => Some('}'),
        'g' | 'k' => reference_delimiter(value),
        _ => None,
    };
    if let Some(close) = delimiter {
        return delimited(text, initial, close);
    }
    control_or_initial(text, initial, ch, value)
}
fn reference_delimiter(value: &str) -> Option<char> {
    match value.as_bytes().get(1) {
        Some(b'<') => Some('>'),
        Some(b'\'') => Some('\''),
        _ => None,
    }
}
fn control_or_initial(text: &str, initial: usize, ch: char, value: &str) -> usize {
    match ch {
        'c' => control_end(text, initial),
        'C' | 'M' if value.as_bytes().get(1) == Some(&b'-') => control_end(text, initial + 1),
        _ => initial,
    }
}

fn delimited(text: &str, start: usize, close: char) -> usize {
    text.get(start + 1..)
        .and_then(|rest| rest.find(close))
        .map(|i| start + 2 + i)
        .unwrap_or(start)
}
fn control_end(text: &str, start: usize) -> usize {
    if text.as_bytes().get(start) == Some(&b'\\') {
        return end(text, start);
    }
    start
        + text[start..]
            .chars()
            .next()
            .map(char::len_utf8)
            .unwrap_or(0)
}
