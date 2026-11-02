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
    let argument = index_argument(n, recv)?;
    Some((recv, index_max(c.text(argument))?, recv.end_byte()))
}
fn index_argument<'n>(n: Node<'n>, recv: Node<'n>) -> Option<Node<'n>> {
    let attached = block(n);
    let mut arguments = kids(n)
        .into_iter()
        .filter(|argument| *argument != recv && Some(*argument) != attached);
    let argument = arguments.next()?;
    arguments.next().is_none().then_some(argument)
}
fn index_max(text: &str) -> Option<bool> {
    match text {
        "0" => Some(false),
        "-1" => Some(true),
        _ => None,
    }
}
fn method_access(c: &Context<'_>, n: Node<'_>) -> Option<bool> {
    let a = semantic_args(n);
    match c.method(n) {
        "first" if a.is_empty() => Some(false),
        "last" if a.is_empty() => Some(true),
        "at" | "slice" | "[]" if a.len() == 1 => index_max(c.text(a[0])),
        _ => None,
    }
}
