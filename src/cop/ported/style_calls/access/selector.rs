use super::super::super::context::*;
use tree_sitter::Node;
pub(super) fn access<'a>(c: &Context<'_>, n: Node<'a>) -> Option<(Node<'a>, bool, usize)> {
    if n.kind() == "element_reference" {
        return index_access(c, n);
    }
    let recv = field(n, "receiver")?;
    Some((
        recv,
        method_access(c, n)?,
        field(n, "operator")
            .map(|o| o.start_byte())
            .unwrap_or(recv.end_byte()),
    ))
}
fn index_access<'a>(c: &Context<'_>, n: Node<'a>) -> Option<(Node<'a>, bool, usize)> {
    let recv = field(n, "object")?;
    let a = kids(n)
        .into_iter()
        .filter(|a| *a != recv)
        .collect::<Vec<_>>();
    if a.len() != 1 {
        return None;
    }
    Some((recv, index_max(c.text(a[0]))?, recv.end_byte()))
}
fn index_max(text: &str) -> Option<bool> {
    match text {
        "0" => Some(false),
        "-1" => Some(true),
        _ => None,
    }
}
fn method_access(c: &Context<'_>, n: Node<'_>) -> Option<bool> {
    let a = args(n);
    match c.method(n) {
        "first" if a.is_empty() => Some(false),
        "last" if a.is_empty() => Some(true),
        "at" | "slice" | "[]" if a.len() == 1 => index_max(c.text(a[0])),
        _ => None,
    }
}
