use super::super::context::*;
use tree_sitter::Node;
pub(super) fn map_compact(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let Some((map, current)) = map_candidate(c, n) else {
        return vec![];
    };
    let Some(positive) = map_block(c, map) else {
        return vec![];
    };
    vec![map_finding(n, map, &current, positive)]
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
    if c.method(n) != "compact" || !semantic_args(n).is_empty() {
        return None;
    }
    let recv = field(n, "receiver").filter(|r| ["map", "filter_map"].contains(&c.method(*r)))?;
    Some((recv, format!("{} {{ ... }}.compact", c.method(recv))))
}
fn map_block(c: &Context<'_>, map: Node<'_>) -> Option<bool> {
    let b = block(map)?;
    let p = field(b, "parameters").map(kids).unwrap_or_default();
    if p.len() != 1 {
        return None;
    }
    conditional_map(c, body(b)?, c.text(p[0])).map(|(_, positive)| positive)
}
fn map_finding(n: Node<'_>, map: Node<'_>, current: &str, positive: bool) -> Finding {
    let method = if positive { "select" } else { "reject" };
    Finding::range(
        method_node(map).start_byte(),
        n.end_byte(),
        format!("Replace `{current}` with `{method}`."),
    )
}

mod condition;
use condition::conditional_map;
