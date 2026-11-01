use super::*;

pub(super) fn unsafe_context(c: &Context<'_>, node: Node<'_>, parameter: &str) -> bool {
    if boundary(node.kind()) {
        return descendants(node)
            .any(|child| child.kind() == "identifier" && c.text(child) == parameter);
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .any(|child| unsafe_context(c, child, parameter))
}
fn boundary(kind: &str) -> bool {
    matches!(
        kind,
        "method" | "singleton_method" | "class" | "module" | "singleton_class" | "in_clause"
    ) || kind.contains("pattern")
}
