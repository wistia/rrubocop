use super::super::super::context::*;
use super::binding;
use tree_sitter::Node;

pub(super) fn assignment(c: &Context<'_>, left: Node<'_>, right: Node<'_>) -> bool {
    let Some((object, key)) = indexed(left) else {
        return false;
    };
    let Some((receiver, read_key)) = getter(c, right) else {
        return false;
    };
    binding::not_a_call(c, key)
        && canonical(object, c) == canonical(receiver, c)
        && canonical(key, c) == canonical(read_key, c)
}
pub(super) fn setter(c: &Context<'_>, n: Node<'_>, arguments: &[Node<'_>]) -> bool {
    if arguments.len() != 2 || !binding::not_a_call(c, arguments[0]) {
        return false;
    }
    let Some((receiver, key)) = getter(c, arguments[1]) else {
        return false;
    };
    c.text(receiver) == c.receiver(n) && canonical(key, c) == canonical(arguments[0], c)
}
fn indexed(n: Node<'_>) -> Option<(Node<'_>, Node<'_>)> {
    let object = field(n, "object")?;
    let keys = kids(n)
        .into_iter()
        .filter(|ch| *ch != object)
        .collect::<Vec<_>>();
    if keys.len() == 1 {
        Some((object, keys[0]))
    } else {
        None
    }
}
fn getter<'a>(c: &Context<'_>, n: Node<'a>) -> Option<(Node<'a>, Node<'a>)> {
    if n.kind() == "element_reference" {
        return element_getter(n);
    }
    if !is_call(n) || c.method(n) != "[]" || safe(c, n) {
        return None;
    }
    Some((field(n, "receiver")?, *args(n).first()?))
}

fn element_getter(n: Node<'_>) -> Option<(Node<'_>, Node<'_>)> {
    let object = field(n, "object")?;
    Some((object, kids(n).into_iter().find(|ch| *ch != object)?))
}
