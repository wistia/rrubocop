use super::super::context::*;
use tree_sitter::Node;
pub(super) fn slicing(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let Some((obj, range)) = slice_range(c, n) else {
        return vec![];
    };
    let span = selector_span(n, obj);
    let selector = String::from_utf8_lossy(&c.source.as_bytes()[span.0..span.1]);
    if range_zero(c, range) && range_tail(c, range) {
        return vec![useless(c, n, obj, &selector)];
    }
    preferred(c, n, obj, range, &selector).into_iter().collect()
}
fn slice_range<'n>(c: &Context<'_>, n: Node<'n>) -> Option<(Node<'n>, Node<'n>)> {
    if is_call(n) {
        return call_range(c, n);
    }
    let obj = field(n, "object")?;
    let a = kids(n)
        .into_iter()
        .filter(|a| *a != obj)
        .collect::<Vec<_>>();
    (a.len() == 1 && a[0].kind() == "range").then(|| (obj, a[0]))
}
fn range_zero(c: &Context<'_>, n: Node<'_>) -> bool {
    field(n, "begin").is_some_and(|b| super::integer::value(c, b) == Some(0.into()))
}
fn range_tail(c: &Context<'_>, n: Node<'_>) -> bool {
    field(n, "end").is_some_and(|e| {
        e.kind() == "nil"
            || operator(c, n) == ".." && super::integer::value(c, e) == Some((-1).into())
    })
}
fn useless(c: &Context<'_>, n: Node<'_>, obj: Node<'_>, selector: &str) -> Finding {
    let (start, end) = selector_span(n, obj);
    let finding = Finding::range(start, end, format!("Remove the useless `{selector}`."));
    if is_call(n) {
        finding
    } else {
        finding.edit_with(c, start, end, || "")
    }
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
            selector_span(n, obj).0,
            selector_span(n, obj).1,
            format!("Prefer `{replacement}` over `{selector}`."),
        )
        .replace_with(c, edit, || ""),
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

fn selector_span(n: Node<'_>, obj: Node<'_>) -> (usize, usize) {
    if is_call(n) {
        let method = method_node(n);
        (method.start_byte(), method.end_byte())
    } else {
        (obj.end_byte(), n.end_byte())
    }
}

fn call_range<'n>(c: &Context<'_>, n: Node<'n>) -> Option<(Node<'n>, Node<'n>)> {
    if c.method(n) != "[]" {
        return None;
    }
    let args = semantic_args(n);
    if args.len() == 1 && args[0].kind() == "range" {
        Some((field(n, "receiver")?, args[0]))
    } else {
        None
    }
}
