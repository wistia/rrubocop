use super::super::context::*;
use tree_sitter::Node;
pub(super) fn map_compact(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let Some((map, current)) = map_candidate(c, n) else {
        return vec![];
    };
    let Some((param, condition, positive)) = map_block(c, map) else {
        return vec![];
    };
    vec![map_finding(
        c,
        n,
        map,
        &current,
        (param, condition, positive),
    )]
}
fn map_candidate<'n>(c: &Context<'_>, n: Node<'n>) -> Option<(Node<'n>, String)> {
    if c.method(n) == "filter_map" {
        return standalone_filter(c, n).then(|| (n, "filter_map { ... }".into()));
    }
    compact_candidate(c, n)
}
fn standalone_filter(c: &Context<'_>, n: Node<'_>) -> bool {
    n.parent().is_none_or(|p| c.method(p) != "compact")
}
fn compact_candidate<'n>(c: &Context<'_>, n: Node<'n>) -> Option<(Node<'n>, String)> {
    if c.method(n) != "compact" || !args(n).is_empty() {
        return None;
    }
    let recv = field(n, "receiver").filter(|r| ["map", "filter_map"].contains(&c.method(*r)))?;
    Some((recv, format!("{} {{ ... }}.compact", c.method(recv))))
}
fn map_block<'n>(c: &Context<'_>, map: Node<'n>) -> Option<(Node<'n>, Node<'n>, bool)> {
    let b = block(map)?;
    let p = field(b, "parameters").map(kids).unwrap_or_default();
    if p.len() != 1 {
        return None;
    }
    let (condition, positive) = conditional_map(c, body(b)?, c.text(p[0]))?;
    Some((p[0], condition, positive))
}
fn map_finding(
    c: &Context<'_>,
    n: Node<'_>,
    map: Node<'_>,
    current: &str,
    (param, condition, positive): (Node<'_>, Node<'_>, bool),
) -> Finding {
    let method = if positive { "select" } else { "reject" };
    let start = method_node(map).start_byte();
    Finding::range(
        start,
        n.end_byte(),
        format!("Replace `{current}` with `{method}`."),
    )
    .edit(
        start,
        n.end_byte(),
        format!("{method} {{ |{}| {} }}", c.text(param), c.text(condition)),
    )
}
mod condition;
use condition::conditional_map;
