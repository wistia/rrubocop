use super::super::context::*;
use super::arguments as args;
use tree_sitter::Node;
mod correction;

pub(super) fn single(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if c.method(n) != "attribute" || block(n).is_some() {
        return vec![];
    }
    let Some(attribute) = args(n).first().copied() else {
        return vec![];
    };
    let Some(method) = matching_method(c, n, attribute) else {
        return vec![];
    };
    vec![correction::single(
        c,
        n,
        method,
        Finding::new(n, "Prefer block attribute over method call."),
    )]
}
pub(super) fn plural(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if c.method(n) != "attributes" || block(n).is_some() {
        return vec![];
    }
    let a = attribute_arguments(n);
    let mut corrected = false;
    a.iter()
        .enumerate()
        .rev()
        .filter_map(|(i, attribute)| {
            let method = matching_method(c, n, *attribute)?;
            Some(plural_finding(c, n, method, &a, i, &mut corrected))
        })
        .collect()
}
fn plural_finding(
    c: &Context<'_>,
    n: Node<'_>,
    method: Node<'_>,
    a: &[Node<'_>],
    i: usize,
    corrected: &mut bool,
) -> Finding {
    let f = Finding::new(
        a[i],
        "Prefer block attribute over method call from attributes.",
    );
    if *corrected {
        return f;
    }
    *corrected = true;
    correction::plural(c, n, method, a, i, f)
}

fn scope(n: Node<'_>) -> Option<Node<'_>> {
    ancestors(n)
        .into_iter()
        .find(|p| matches!(p.kind(), "class" | "module" | "singleton_class"))
}
fn matching_method<'n>(c: &Context<'_>, n: Node<'n>, attribute: Node<'_>) -> Option<Node<'n>> {
    if !c.symbol(attribute) {
        return None;
    }
    let owner = scope(n)?;
    descendants(owner).into_iter().find(|m| {
        m.kind() == "method" && c.method(*m) == c.value(attribute) && scope(*m) == Some(owner)
    })
}

fn attribute_arguments(n: Node<'_>) -> Vec<Node<'_>> {
    args(n)
        .into_iter()
        .filter(|n| n.kind() != "comment")
        .collect()
}
