use super::super::context::*;
use tree_sitter::Node;
pub(super) fn dir(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if !dir_candidate(c, n) {
        return vec![];
    }
    vec![
        Finding::new(
            n,
            "Use `__dir__` to get an absolute path to the current file's directory.",
        )
        .replace(n, "__dir__"),
    ]
}
fn file_receiver(c: &Context<'_>, n: Node<'_>) -> bool {
    ["File", "::File"].contains(&c.receiver(n))
}
fn dir_candidate(c: &Context<'_>, n: Node<'_>) -> bool {
    if !file_receiver(c, n) {
        return false;
    }
    let a = args(n);
    a.len() == 1 && dir_inner(c, n, a[0])
}
fn dir_inner(c: &Context<'_>, outer: Node<'_>, inner: Node<'_>) -> bool {
    file_receiver(c, inner)
        && [("expand_path", "dirname"), ("dirname", "realpath")]
            .contains(&(c.method(outer), c.method(inner)))
        && file_argument(c, inner)
}
fn file_argument(c: &Context<'_>, n: Node<'_>) -> bool {
    let a = args(n);
    a.len() == 1 && c.text(a[0]) == "__FILE__"
}
pub(super) fn string_chars(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if c.method(n) != "split"
        || args(n).len() != 1
        || !["//", "''", "\"\""].contains(&c.text(args(n)[0]))
    {
        return vec![];
    }
    let start = method_node(n).start_byte();
    let current = String::from_utf8_lossy(&c.source.as_bytes()[start..n.end_byte()]);
    vec![
        Finding::range(
            start,
            n.end_byte(),
            format!("Use `chars` instead of `{current}`."),
        )
        .edit(start, n.end_byte(), "chars"),
    ]
}
pub(super) fn stderr(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if c.method(n) != "puts"
        || args(n).is_empty()
        || !["$stderr", "STDERR", "::STDERR"].contains(&c.receiver(n))
    {
        return vec![];
    }
    let end = method_node(n).end_byte();
    vec![
        Finding::range(
            n.start_byte(),
            end,
            format!(
                "Use `warn` instead of `{}.puts` to allow such output to be disabled.",
                c.receiver(n)
            ),
        )
        .edit(n.start_byte(), end, "warn"),
    ]
}
pub(super) fn require_extension(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let Some(a) = require_argument(c, n) else {
        return vec![];
    };
    if a.kind() != "string"
        || descendants(a).iter().any(|d| d.kind() == "interpolation")
        || !c.value(a).ends_with(".rb")
    {
        return vec![];
    }
    let start = a.end_byte() - 4;
    vec![
        Finding::range(start, start + 3, "Redundant `.rb` file extension detected.").edit(
            start,
            start + 3,
            "",
        ),
    ]
}
fn require_argument<'n>(c: &Context<'_>, n: Node<'n>) -> Option<Node<'n>> {
    if !["require", "require_relative"].contains(&c.method(n)) || !c.receiver(n).is_empty() {
        return None;
    }
    let a = args(n);
    (a.len() == 1).then(|| a[0])
}
