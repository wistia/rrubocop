use super::super::context::*;
use tree_sitter::Node;

pub(super) fn redis_expiry(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if c.method(n) != "set" {
        return vec![];
    }
    if !c.receiver(n).to_lowercase().contains("redis") && !redis_block(c, n) {
        return vec![];
    }
    if has_expiration(c, n) {
        return vec![];
    }
    vec![Finding::new(
        n,
        "`$redis.set` must be called with an expiration option set (ex, exat, px, or pxat).",
    )]
}

fn expiration(c: &Context<'_>, n: Node<'_>) -> bool {
    ["ex", "exat", "px", "pxat"]
        .iter()
        .any(|key| c.symbol_pair_value(n, key).is_some())
}

fn redis_block(c: &Context<'_>, n: Node<'_>) -> bool {
    if n.parent()
        .is_none_or(|p| p.kind() != "block_body" || kids(p).len() < 2)
    {
        return false;
    }
    ancestors(n)
        .into_iter()
        .filter(|parent| transaction(c, *parent))
        .any(|parent| {
            first_parameter(parent).is_some_and(|parameter| c.text(parameter) == c.receiver(n))
        })
}

fn transaction(c: &Context<'_>, n: Node<'_>) -> bool {
    is_call(n)
        && matches!(c.method(n), "multi" | "pipelined")
        && c.text(n).to_lowercase().contains("redis")
}

fn first_parameter(n: Node<'_>) -> Option<Node<'_>> {
    block(n)
        .and_then(|b| b.child_by_field_name("parameters"))
        .and_then(|parameters| kids(parameters).first().copied())
}

fn has_expiration(c: &Context<'_>, n: Node<'_>) -> bool {
    args(n)
        .get(2)
        .is_some_and(|argument| expiration(c, *argument))
        || expiration(c, n)
}
