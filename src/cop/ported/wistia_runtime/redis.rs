use super::super::context::*;
use tree_sitter::Node;

pub(super) fn redis_expiry(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if c.method(n) != "set" || has_expiration(c, n) {
        return vec![];
    }
    if !c.receiver(n).to_lowercase().contains("redis") && !redis_block(c, n) {
        return vec![];
    }
    vec![Finding::new(
        n,
        "`$redis.set` must be called with an expiration option set (ex, exat, px, or pxat).",
    )]
}

fn expiration(c: &Context<'_>, n: Node<'_>) -> bool {
    option_pairs(n).any(|pair| {
        field(pair, "key").is_some_and(|key| {
            c.symbol(key) && ["ex", "exat", "px", "pxat"].contains(&c.value(key))
        })
    })
}

fn redis_block(c: &Context<'_>, n: Node<'_>) -> bool {
    c.source
        .cached(0, || transactions::Transactions::new(c, n))
        .contains(n)
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
        .into_iter()
        .filter(|a| a.kind() != "comment")
        .skip(2)
        .any(|option| expiration(c, option))
}

fn transaction_body(n: Node<'_>) -> bool {
    matches!(n.kind(), "block_body" | "body_statement") && {
        let mut cursor = n.walk();
        n.named_children(&mut cursor)
            .filter(|n| n.kind() != "comment")
            .take(2)
            .count()
            == 2
    }
}

#[cfg(test)]
#[path = "guardrail_tests.rs"]
mod tests;

mod transactions;
