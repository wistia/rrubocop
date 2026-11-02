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
    c.method(n) == "unpack"
        && semantic_args(n).len() == 1
        && !c.receiver(n).is_empty()
        && block(n).is_none()
}
fn unpack_finding(c: &Context<'_>, n: Node<'_>, recv: Node<'_>) -> Finding {
    let start = method_node(recv).start_byte();
    let end = accessor_end(c, n);
    let current = String::from_utf8_lossy(&c.source.as_bytes()[start..end]);
    Finding::range(
        start,
        end,
        format!(
            "Use `unpack1({})` instead of `{current}`.",
            c.text(semantic_args(recv)[0])
        ),
    )
    .edit_with(c, start, end, || {
        String::from_utf8_lossy(&c.source.as_bytes()[start..recv.end_byte()])
            .replacen("unpack", "unpack1", 1)
    })
}
fn accessor_end(c: &Context<'_>, n: Node<'_>) -> usize {
    let source = &c.text(n)[..send_end(n) - n.start_byte()];
    n.start_byte()
        + source
            .trim_end_matches(|character: char| character.is_ascii_whitespace())
            .len()
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
        "sort" => semantic_args(n).is_empty(),
        "sort_by" => semantic_args(n).len() == 1 || block(n).is_some(),
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
    let end = accessor_end(c, n);
    let inside =
        String::from_utf8_lossy(&c.source.as_bytes()[selector.end_byte()..recv.end_byte()]);
    Finding::range(
        selector.start_byte(),
        end,
        sort_message(c, n, recv, start, suggestion),
    )
    .edit_with(c, selector.start_byte(), end, || {
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
    if block(n).is_some() {
        return None;
    }
    let parent = n
        .parent()
        .filter(|p| p.kind() == "binary" && ["&&", "||", "and", "or"].contains(&operator(c, *p)))?;
    field(parent, "operator")
}

fn accessor_source<'c>(c: &'c Context<'_>, n: Node<'_>, start: usize) -> std::borrow::Cow<'c, str> {
    String::from_utf8_lossy(&c.source.as_bytes()[start..accessor_end(c, n)])
}

fn logical_suffix(c: &Context<'_>, n: Node<'_>) -> String {
    logical_operator(c, n)
        .map(|operator| format!(" {}", c.text(operator)))
        .unwrap_or_default()
}
