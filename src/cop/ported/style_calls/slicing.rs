use super::super::context::*;
use tree_sitter::Node;
pub(super) fn slicing(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let Some((obj, range)) = slice_range(n) else {
        return vec![];
    };
    let selector = String::from_utf8_lossy(&c.source.as_bytes()[obj.end_byte()..n.end_byte()]);
    if range_zero(c, range) && range_tail(c, range) {
        return vec![useless(n, obj, &selector)];
    }
    preferred(c, n, obj, range, &selector).into_iter().collect()
}
fn slice_range(n: Node<'_>) -> Option<(Node<'_>, Node<'_>)> {
    let obj = field(n, "object")?;
    let a = kids(n)
        .into_iter()
        .filter(|a| *a != obj)
        .collect::<Vec<_>>();
    (a.len() == 1 && a[0].kind() == "range").then(|| (obj, a[0]))
}
fn range_zero(c: &Context<'_>, n: Node<'_>) -> bool {
    field(n, "begin").is_some_and(|b| c.text(b) == "0")
}
fn range_tail(c: &Context<'_>, n: Node<'_>) -> bool {
    field(n, "end")
        .is_some_and(|e| e.kind() == "nil" || operator(c, n) == ".." && c.text(e) == "-1")
}
fn useless(n: Node<'_>, obj: Node<'_>, selector: &str) -> Finding {
    Finding::range(
        obj.end_byte(),
        n.end_byte(),
        format!("Remove the useless `{selector}`."),
    )
    .edit(obj.end_byte(), n.end_byte(), "")
}
fn preferred(
    c: &Context<'_>,
    n: Node<'_>,
    obj: Node<'_>,
    range: Node<'_>,
    selector: &str,
) -> Option<Finding> {
    let (replacement, edit) = preferred_edit(c, range)?;
    Some(
        Finding::range(
            obj.end_byte(),
            n.end_byte(),
            format!("Prefer `{replacement}` over `{selector}`."),
        )
        .replace(edit, ""),
    )
}
fn preferred_edit<'n>(c: &Context<'_>, range: Node<'n>) -> Option<(String, Node<'n>)> {
    match (field(range, "begin"), field(range, "end")) {
        (Some(begin), Some(end))
            if begin.kind() == "nil"
                && operator(c, range) == ".."
                && c.config.get_f64("TargetRubyVersion", 2.7) >= 2.7 =>
        {
            Some((format!("[..{}]", c.text(end)), begin))
        }
        (Some(begin), Some(end)) if range_tail(c, range) => {
            Some((format!("[{}{}]", c.text(begin), operator(c, range)), end))
        }
        _ => None,
    }
}
