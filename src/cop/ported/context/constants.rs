use super::*;
use std::borrow::Cow;

pub(super) fn name<'a>(c: &Context<'a>, node: Node<'_>) -> Option<Cow<'a, str>> {
    match node.kind() {
        "constant" => Some(Cow::Borrowed(c.text(node))),
        "scope_resolution" => scoped(c, node),
        _ => None,
    }
}

fn scoped<'a>(c: &Context<'a>, node: Node<'_>) -> Option<Cow<'a, str>> {
    let leaf = field(node, "name").filter(|leaf| leaf.kind() == "constant")?;
    let Some(scope) = field(node, "scope") else {
        return Some(Cow::Borrowed(c.text(leaf)));
    };
    Some(join(name(c, scope)?, c.text(leaf), c.text(node)))
}

fn join<'a>(prefix: Cow<'a, str>, leaf: &'a str, source: &'a str) -> Cow<'a, str> {
    let text = source.trim_start_matches("::");
    if text
        .strip_prefix(prefix.as_ref())
        .and_then(|rest| rest.strip_prefix("::"))
        == Some(leaf)
    {
        Cow::Borrowed(text)
    } else {
        Cow::Owned(format!("{prefix}::{leaf}"))
    }
}
