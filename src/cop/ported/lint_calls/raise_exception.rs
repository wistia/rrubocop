use super::super::context::*;
use tree_sitter::Node;

pub(super) fn raise_exception(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let Some(exception) = raised_exception(c, n) else {
        return vec![];
    };
    if !root_constant(c, exception, "Exception") || implicit_namespace(c, n, exception) {
        return vec![];
    }
    let replacement = if c.text(exception).starts_with("::") {
        "::StandardError"
    } else {
        "StandardError"
    };
    vec![
        Finding::new(exception, "Use `StandardError` over `Exception`.")
            .replace(exception, replacement),
    ]
}
fn raised_exception<'a>(c: &Context<'_>, n: Node<'a>) -> Option<Node<'a>> {
    if !matches!(c.method(n), "raise" | "fail") || !c.receiver(n).is_empty() {
        return None;
    }
    let exception = *args(n).first()?;
    if is_call(exception) && c.method(exception) == "new" {
        field(exception, "receiver")
    } else {
        Some(exception)
    }
}
fn implicit_namespace(c: &Context<'_>, n: Node<'_>, exception: Node<'_>) -> bool {
    if c.text(exception).starts_with("::") {
        return false;
    }
    ancestors(n)
        .into_iter()
        .filter(|p| p.kind() == "module")
        .filter_map(|p| field(p, "name"))
        .any(|name| allowed_namespace(c, c.text(name)))
}
fn allowed_namespace(c: &Context<'_>, namespace: &str) -> bool {
    c.config
        .options
        .get("AllowedImplicitNamespaces")
        .and_then(|v| v.as_sequence())
        .is_some_and(|allowed| allowed.iter().any(|v| v.as_str() == Some(namespace)))
}
