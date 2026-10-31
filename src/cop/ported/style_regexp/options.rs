pub(super) fn closing(ch: char) -> char {
    match ch {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        '<' => '>',
        c => c,
    }
}
pub(super) fn scoped(text: &str, start: usize, initial: bool) -> Option<(usize, bool, bool)> {
    let options = text.get(start + 2..)?;
    if !text[start..].starts_with("(?") {
        return None;
    }
    let end = option_end(options)?;
    if end == 0 || !matches!(options.as_bytes()[end], b':' | b')') {
        return None;
    }
    Some((
        start + 2 + end + 1,
        extended(&options[..end], initial),
        options.as_bytes()[end] == b':',
    ))
}
fn extended(options: &str, initial: bool) -> bool {
    let mut value = initial;
    let mut enabled = true;
    for flag in options.chars() {
        match flag {
            '-' => enabled = false,
            'x' => value = enabled,
            _ => {}
        }
    }
    value
}
pub(super) fn comment_end(text: &str, start: usize) -> usize {
    let mut escaped = false;
    for (i, ch) in text[start..].char_indices() {
        if !escaped && ch == ')' {
            return start + i + 1;
        }
        escaped = !escaped && ch == '\\';
    }
    text.len()
}

fn option_end(options: &str) -> Option<usize> {
    options.find(|c: char| !"imxadu-".contains(c))
}
