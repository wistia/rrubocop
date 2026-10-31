use super::super::super::context::*;
use tree_sitter::Node;
pub(super) fn single(c: &Context<'_>, n: Node<'_>, method: Node<'_>, f: Finding) -> Finding {
    let Some(body) = field(method, "body") else {
        return f;
    };
    let replacement = format!(
        "{} do\n{}{}\n{}end",
        c.text(n),
        " ".repeat(body.start_position().column),
        c.text(body),
        " ".repeat(n.start_position().column)
    );
    f.replace(n, replacement).also_edit(
        leading_whitespace(c, method.start_byte()),
        method.end_byte(),
        "",
    )
}
pub(super) fn plural(
    c: &Context<'_>,
    n: Node<'_>,
    method: Node<'_>,
    a: &[Node<'_>],
    i: usize,
    f: Finding,
) -> Finding {
    let replacement = c.text(method).replacen(
        &format!("def {}", c.value(a[i])),
        &format!("attribute :{} do", c.value(a[i])),
        1,
    );
    let (start, end) = remove_attribute(c, n, a, i);
    f.replace(method, replacement).also_edit(start, end, "")
}
fn remove_attribute(c: &Context<'_>, n: Node<'_>, a: &[Node<'_>], i: usize) -> (usize, usize) {
    if a.len() == 1 {
        return (n.start_byte(), trailing_whitespace(c, n.end_byte()));
    }
    if i == 0 {
        return (a[0].start_byte(), a[1].start_byte());
    }
    let left = a[i - 1].end_byte();
    (
        left + c.source.as_bytes()[left..a[i].start_byte()]
            .iter()
            .position(|b| *b == b',')
            .unwrap(),
        a[i].end_byte(),
    )
}
fn leading_whitespace(c: &Context<'_>, mut start: usize) -> usize {
    let bytes = c.source.as_bytes();
    while start > 0 && bytes[start - 1].is_ascii_whitespace() {
        start -= 1;
    }
    start
}
fn trailing_whitespace(c: &Context<'_>, mut end: usize) -> usize {
    let bytes = c.source.as_bytes();
    while end < bytes.len() && bytes[end].is_ascii_whitespace() {
        end += 1;
    }
    end
}
