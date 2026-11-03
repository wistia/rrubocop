//! Continuation / open-delimiter line detection for IndentationWidth.

pub(super) fn ends_with_open_delim(line: &[u8]) -> bool {
    if line_has_unclosed_open(line) {
        return true;
    }
    let code = strip_line_comment(line);
    let mut i = code.len();
    while i > 0 {
        i -= 1;
        match code[i] {
            b' ' | b'\t' | b'\r' => continue,
            b'(' | b'[' | b'{' => return true,
            _ => return false,
        }
    }
    false
}

fn line_has_unclosed_open(line: &[u8]) -> bool {
    let code = strip_line_comment(line);
    let mut depth = 0i32;
    for &b in code {
        match b {
            b'{' | b'(' | b'[' => depth += 1,
            b'}' | b')' | b']' => depth -= 1,
            _ => {}
        }
    }
    depth > 0
}

fn strip_line_comment(line: &[u8]) -> &[u8] {
    match crate::parse::comment_hash::first_comment_hash(line) {
        Some(i) => &line[..i],
        None => line,
    }
}

fn ends_with_multi_char_op(t: &[u8]) -> bool {
    const OPS: &[&[u8]] = &[
        b"->", b"=>", b"&&", b"||", b"==", b"!=", b">=", b"<=", b"<<", b">>",
    ];
    OPS.iter().any(|op| t.ends_with(op))
}

fn ends_with_single_char_op(t: &[u8]) -> bool {
    matches!(
        t.last(),
        Some(
            b',' | b'\\'
                | b'('
                | b'['
                | b'{'
                | b'+'
                | b'-'
                | b'*'
                | b'/'
                | b'%'
                | b'|'
                | b'^'
                | b'<'
                | b'>'
                | b'.'
        )
    )
}

pub(super) fn ends_with_continuation(line: &[u8]) -> bool {
    let t = trim_ascii_end(strip_line_comment(line));
    !t.is_empty()
        && (ends_with_multi_char_op(t)
            || ends_with_single_char_op(t)
            || trailing_if_kw(t)
            || case_opener(t))
}

fn case_opener(t: &[u8]) -> bool {
    let Ok(s) = std::str::from_utf8(t) else {
        return false;
    };
    let s = s.trim_end();
    s.starts_with("case ")
        || s.starts_with("case(")
        || s.contains(" = case ")
        || s.contains("=case ")
        || s.ends_with("= case")
}

fn trim_ascii_end(code: &[u8]) -> &[u8] {
    let mut end = code.len();
    while end > 0 && matches!(code[end - 1], b' ' | b'\t' | b'\r') {
        end -= 1;
    }
    &code[..end]
}

fn trailing_if_kw(t: &[u8]) -> bool {
    let Ok(s) = std::str::from_utf8(t) else {
        return false;
    };
    let s = s.trim_end();
    if s.ends_with("then") || s.ends_with("end") {
        return false;
    }
    let st = s.trim_start();
    s.contains(" if ")
        || s.contains(" unless ")
        || st.starts_with("if ")
        || st.starts_with("unless ")
        || st.starts_with("else")
}

/// Indent of `.find_each do |x|` / `.map do` when the next line is the body.
pub(super) fn deeper_dot_block(
    line: &[u8],
    next_indent: impl FnOnce() -> Option<usize>,
) -> Option<usize> {
    let indent = line
        .iter()
        .take_while(|&&b| b == b' ' || b == b'\t')
        .count();
    let rest = &line[indent..];
    if !(rest.starts_with(b".") || rest.starts_with(b"&.")) {
        return None;
    }
    let t = trim_ascii_end(rest);
    if !(t.ends_with(b" do") || t.ends_with(b"|")) {
        return None;
    }
    next_indent().filter(|n| *n > indent)?;
    Some(indent)
}

pub(super) fn aligned_continuation(
    indent: usize,
    prev: usize,
    prev_line: &[u8],
    cont_base: &mut Option<usize>,
) -> bool {
    let start =
        indent > prev && (ends_with_open_delim(prev_line) || ends_with_continuation(prev_line));
    if start || cont_base.is_some_and(|b| indent >= b.saturating_sub(1)) {
        if cont_base.is_none() {
            *cont_base = Some(prev);
        }
        true
    } else {
        *cont_base = None;
        false
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    #[test]
    fn dot_block_lookahead_is_lazy_and_retains_lexical_eligibility() {
        for line in [
            b"# comment".as_slice(),
            b"",
            b"  \r",
            b"ordinary do",
            b"  .ordinary",
            b"  &.ordinary",
            b"\xff",
        ] {
            assert_eq!(
                deeper_dot_block(line, || panic!("ordinary lines must not look ahead")),
                None
            );
        }
        for line in [b"  .each do".as_slice(), b"  &.each { |x|", b"\t.each do\r"] {
            let count = std::cell::Cell::new(0);
            let result = deeper_dot_block(line, || {
                count.set(count.get() + 1);
                Some(4)
            });
            assert!(result.is_some());
            assert_eq!(count.get(), 1);
            assert_eq!(deeper_dot_block(line, || Some(0)), None);
            assert_eq!(deeper_dot_block(line, || None), None);
        }
    }
}
