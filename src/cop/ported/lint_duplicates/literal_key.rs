use super::*;

pub(super) fn literal_key(c: &Context<'_>, n: Node<'_>) -> String {
    let n = unwrapped_pattern(n);
    if let Some(value) = c.static_symbol(n) {
        return format!("symbol:{value:?}");
    }
    matches!(n.kind(), "string" | "character")
        .then(|| c.ruby_static_string(n))
        .flatten()
        .map(|value| format!("string:{value:?}"))
        .unwrap_or_else(|| canonical(n, c))
}
fn unwrapped_pattern(n: Node<'_>) -> Node<'_> {
    if n.kind() == "pattern" {
        n.named_child(0).unwrap_or(n)
    } else {
        n
    }
}
