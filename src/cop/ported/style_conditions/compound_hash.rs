use super::super::context::*;
use tree_sitter::Node;
pub(super) fn compound_hash(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if outer_combinator(c, n) {
        return vec![Finding::new(
            n,
            "Use `[...].hash` instead of combining hash values manually.",
        )];
    }
    if c.method(n) != "hash" {
        return vec![];
    }
    hash_message(c, n)
        .map(|message| vec![Finding::new(n, message)])
        .unwrap_or_default()
}
fn combinator(c: &Context<'_>, n: Node<'_>) -> bool {
    matches!(n.kind(), "binary" | "operator_assignment")
        && ["^", "+", "*", "|", "^=", "+=", "*=", "|="].contains(&operator(c, n))
}
fn outer_combinator(c: &Context<'_>, n: Node<'_>) -> bool {
    combinator(c, n)
        && !ancestors(n).any(|p| combinator(c, p))
        && ancestors(n).any(|p| hash_definition(c, p))
}
fn hash_message(c: &Context<'_>, n: Node<'_>) -> Option<&'static str> {
    if field(n, "receiver").is_some_and(|r| r.kind() == "array" && kids(r).len() == 1) {
        return Some(
            "Delegate hash directly without wrapping in an array when only using a single value.",
        );
    }
    if n.parent()
        .is_some_and(|a| a.kind() == "array" && a.parent().is_some_and(|p| c.method(p) == "hash"))
    {
        return Some("Calling .hash on elements of a hashed array is redundant.");
    }
    None
}
fn no_params(n: Node<'_>) -> bool {
    field(n, "parameters").is_none_or(|p| kids(p).is_empty())
}
fn hash_definition(c: &Context<'_>, n: Node<'_>) -> bool {
    if matches!(n.kind(), "method" | "singleton_method") {
        return c.method(n) == "hash" && no_params(n);
    }
    dynamic_hash(c, n) && block(n).is_some_and(no_params)
}
fn dynamic_hash(c: &Context<'_>, n: Node<'_>) -> bool {
    is_call(n)
        && ["define_method", "define_singleton_method"].contains(&c.method(n))
        && args(n).first().is_some_and(|a| c.value(*a) == "hash")
}
