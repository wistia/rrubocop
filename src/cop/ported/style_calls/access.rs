use super::super::context::*;
use tree_sitter::Node;
mod selector;
use selector::access;
pub(super) fn unpack(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let Some((recv, max, _)) = access(c, n) else {
        return vec![];
    };
    if max || !unpack_candidate(c, recv) {
        return vec![];
    }
    vec![unpack_finding(c, n, recv)]
}
fn unpack_candidate(c: &Context<'_>, n: Node<'_>) -> bool {
    c.method(n) == "unpack" && !semantic_args(n).is_empty() && !c.receiver(n).is_empty()
}
fn unpack_finding(c: &Context<'_>, n: Node<'_>, recv: Node<'_>) -> Finding {
    let start = method_node(recv).start_byte();
    let current = String::from_utf8_lossy(&c.source.as_bytes()[start..n.end_byte()]);
    Finding::range(
        start,
        n.end_byte(),
        format!(
            "Use `unpack1({})` instead of `{current}`.",
            c.text(semantic_args(recv)[0])
        ),
    )
    .edit_with(c, start, n.end_byte(), || {
        String::from_utf8_lossy(&c.source.as_bytes()[start..recv.end_byte()])
            .replacen("unpack", "unpack1", 1)
    })
}
pub(super) fn sort(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let Some((recv, max, access_start)) = access(c, n) else {
        return vec![];
    };
    if !sort_candidate(c, recv) {
        return vec![];
    }
    vec![sort_finding(c, n, recv, max, access_start)]
}
fn sort_candidate(c: &Context<'_>, n: Node<'_>) -> bool {
    match c.method(n) {
        "sort" => args(n).is_empty(),
        "sort_by" => args(n).len() == 1 || block(n).is_some(),
        _ => false,
    }
}
fn sort_suggestion(sorter: &str, max: bool) -> String {
    format!(
        "{}{}",
        if max { "max" } else { "min" },
        if sorter == "sort_by" { "_by" } else { "" }
    )
}
fn sort_finding(c: &Context<'_>, n: Node<'_>, recv: Node<'_>, max: bool, start: usize) -> Finding {
    let finding = sort_edit(c, n, recv, start, &sort_suggestion(c.method(recv), max));
    if let Some(op) = logical_operator(c, n) {
        return finding.also_edit_with(c, op.start_byte(), op.end_byte(), || "");
    }
    finding
}
fn sort_edit(
    c: &Context<'_>,
    n: Node<'_>,
    recv: Node<'_>,
    start: usize,
    suggestion: &str,
) -> Finding {
    let selector = method_node(recv);
    let inside =
        String::from_utf8_lossy(&c.source.as_bytes()[selector.end_byte()..recv.end_byte()]);
    Finding::range(
        selector.start_byte(),
        n.end_byte(),
        sort_message(c, n, recv, start, suggestion),
    )
    .edit_with(c, selector.start_byte(), n.end_byte(), || {
        format!(
            "{suggestion}{inside}{}{}",
            logical_suffix(c, n),
            String::from_utf8_lossy(&c.source.as_bytes()[recv.end_byte()..start])
        )
    })
}
fn sort_message(
    c: &Context<'_>,
    n: Node<'_>,
    recv: Node<'_>,
    start: usize,
    suggestion: &str,
) -> String {
    format!(
        "Use `{suggestion}` instead of `{}...{}`.",
        c.method(recv),
        accessor_source(c, n, start).trim_start_matches(['.', '&'])
    )
}
fn logical_operator<'n>(c: &Context<'_>, n: Node<'n>) -> Option<Node<'n>> {
    let parent = n
        .parent()
        .filter(|p| p.kind() == "binary" && ["&&", "||", "and", "or"].contains(&operator(c, *p)))?;
    field(parent, "operator")
}

fn accessor_source<'c>(c: &'c Context<'_>, n: Node<'_>, start: usize) -> std::borrow::Cow<'c, str> {
    String::from_utf8_lossy(&c.source.as_bytes()[start..n.end_byte()])
}

fn logical_suffix(c: &Context<'_>, n: Node<'_>) -> String {
    n.parent()
        .filter(|p| p.kind() == "binary" && ["&&", "||", "and", "or"].contains(&operator(c, *p)))
        .map(|p| format!(" {}", operator(c, p)))
        .unwrap_or_default()
}
