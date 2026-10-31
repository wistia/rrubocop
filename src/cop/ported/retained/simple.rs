use super::super::context::*;
use tree_sitter::Node;

mod setex;
pub(super) use setex::setex;
pub(super) fn refinement(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if c.config.get_f64("TargetRubyVersion", 3.4) < 3.1
        || !matches!(c.method(n), "include" | "prepend")
        || !c.receiver(n).is_empty()
    {
        return vec![];
    }
    if !refine_body(c, n) {
        return vec![];
    }
    vec![Finding::new(
        method_node(n),
        format!(
            "Use `import_methods` instead of `{}` because it is deprecated in Ruby 3.1.",
            c.method(n)
        ),
    )]
}
fn refine_body(c: &Context<'_>, n: Node<'_>) -> bool {
    let Some(p) = n.parent() else {
        return false;
    };
    let p = if p.kind() == "body_statement" {
        p.parent().unwrap_or(p)
    } else {
        p
    };
    matches!(p.kind(), "block" | "do_block")
        && kids(field(p, "body").unwrap_or(p)).len() <= 1
        && p.parent().is_some_and(|call| c.method(call) == "refine")
}
pub(super) fn colon(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let mut cursor = n.walk();
    n.children(&mut cursor)
        .filter(|o| c.text(*o) == "::")
        .map(|o| {
            Finding::new(o, "Do not use `::` for defining class methods.").replace_with(
                c,
                o,
                || ".",
            )
        })
        .collect()
}
pub(super) fn empty_parameters(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let Some(p) = field(n, "parameters").filter(|p| empty_pipes(c, n, *p)) else {
        return vec![];
    };
    let start = n.start_byte() + if n.kind() == "do_block" { 2 } else { 1 };
    vec![
        Finding::new(p, "Omit pipes for the empty block parameters.").edit_with(
            c,
            start,
            p.end_byte(),
            || "",
        ),
    ]
}
fn empty_pipes(c: &Context<'_>, n: Node<'_>, p: Node<'_>) -> bool {
    kids(p).is_empty()
        && c.text(p).contains('|')
        && !n.parent().is_some_and(|p| p.kind() == "lambda")
}
pub(super) fn when_then(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let Some(t) = kids(n).into_iter().find(|ch| ch.kind() == "then") else {
        return vec![];
    };
    let Some(keyword) = then_keyword(t) else {
        return vec![];
    };
    if requires_then(n, t) {
        return vec![];
    }
    vec![
        Finding::new(keyword, "Do not use `then` for multiline `when` statement.").edit_with(
            c,
            before_spaces(c, keyword.start_byte()),
            keyword.end_byte(),
            || "",
        ),
    ]
}
fn then_keyword(n: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = n.walk();
    n.children(&mut cursor)
        .find(|ch| ch.kind() == "then" && !ch.is_named())
}
fn requires_then(n: Node<'_>, t: Node<'_>) -> bool {
    multiline_conditions(n)
        || kids(t)
            .iter()
            .find(|b| b.kind() != "empty_statement")
            .is_some_and(|b| b.start_position().row == n.start_position().row)
}
fn multiline_conditions(n: Node<'_>) -> bool {
    let conditions = kids(n)
        .into_iter()
        .filter(|ch| ch.kind() != "then")
        .collect::<Vec<_>>();
    conditions
        .first()
        .zip(conditions.last())
        .is_some_and(|(a, b)| a.start_position().row != b.end_position().row)
}
fn before_spaces(c: &Context<'_>, mut start: usize) -> usize {
    while start > 0 && matches!(c.source.as_bytes()[start - 1], b' ' | b'\t') {
        start -= 1;
    }
    start
}
