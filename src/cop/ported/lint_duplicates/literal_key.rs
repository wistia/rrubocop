use super::*;
mod numbers;
mod strings;
mod tree;
#[derive(Clone, Copy, PartialEq, Eq)]
enum Equality {
    Node,
    Printed,
}

pub(super) fn literal_key(c: &Context<'_>, n: Node<'_>) -> String {
    semantic_key(c, n, Equality::Node)
}
pub(super) fn require_key(c: &Context<'_>, n: Node<'_>) -> String {
    semantic_key(c, n, Equality::Printed)
}
fn semantic_key(c: &Context<'_>, n: Node<'_>, equality: Equality) -> String {
    let n = unwrapped_pattern(n);
    strings::key(c, n)
        .or_else(|| numbers::key(c, n, equality))
        .unwrap_or_else(|| tree::key(c, n, equality))
}
fn unwrapped_pattern(n: Node<'_>) -> Node<'_> {
    if n.kind() == "pattern" {
        n.named_child(0).unwrap_or(n)
    } else {
        n
    }
}

#[cfg(test)]
mod tests;
