mod empty_when;
use super::context::*;
use crate::cop::CopRegistry;
use empty_when::empty_when;
use std::collections::HashSet;
use tree_sitter::Node;
pub(super) fn register(r: &mut CopRegistry) {
    super::rules!(r;
        ("Lint/DuplicateCaseCondition", &["case"], case_conditions, false, true, false),
        ("Lint/DuplicateElsifCondition", &["if"], elsif_conditions, false, true, false),
        ("Lint/DuplicateRescueException", &["body_statement","begin"], rescue_exceptions, false, true, false),
        ("Lint/DuplicateRequire", &["call"], require, true, false, false),
        ("Lint/EmptyWhen", &["when"], empty_when, false, true, false)
    );
}
fn duplicate(c: &Context<'_>, nodes: Vec<Node<'_>>, msg: &str) -> Vec<Finding> {
    let mut seen = HashSet::new();
    nodes
        .into_iter()
        .filter(|n| !seen.insert(canonical(*n, c)))
        .map(|n| Finding::new(n, msg))
        .collect()
}
fn case_conditions(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    duplicate(
        c,
        kids(n)
            .into_iter()
            .filter(|w| w.kind() == "when")
            .flat_map(|w| {
                let mut cursor = w.walk();
                w.children_by_field_name("pattern", &mut cursor)
                    .collect::<Vec<_>>()
            })
            .collect(),
        "Duplicate `when` condition detected.",
    )
}
fn elsif_conditions(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let mut nodes = vec![];
    let mut next = Some(n);
    while let Some(node) = next {
        if let Some(cond) = field(node, "condition") {
            nodes.push(cond);
        }
        next = field(node, "alternative").filter(|p| p.kind() == "elsif");
    }
    duplicate(c, nodes, "Duplicate `elsif` condition detected.")
}
fn rescue_exceptions(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    duplicate(
        c,
        kids(n)
            .into_iter()
            .filter(|r| r.kind() == "rescue")
            .filter_map(|r| field(r, "exceptions"))
            .flat_map(kids)
            .collect(),
        "Duplicate `rescue` exception detected.",
    )
}
fn require(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if !require_call(c, n) || !repeated_require(c, n) {
        return vec![];
    }
    let (start, end) = whole_lines(c, n);
    vec![Finding::new(n, format!("Duplicate `{}` detected.", c.method(n))).edit(start, end, "")]
}
fn require_call(c: &Context<'_>, n: Node<'_>) -> bool {
    ["require", "require_relative"].contains(&c.method(n))
        && kernel_receiver(c, n)
        && args(n).len() == 1
}
fn repeated_require(c: &Context<'_>, n: Node<'_>) -> bool {
    let parent = ruby_parent(n);
    descendants(parent)
        .into_iter()
        .take_while(|p| *p != n)
        .filter(|p| is_call(*p) && ruby_parent(*p) == parent)
        .any(|p| same_requirement(c, p, n))
}
fn ruby_parent(n: Node<'_>) -> Node<'_> {
    let parent = n.parent().unwrap_or(n);
    if matches!(parent.kind(), "then" | "else") && kids(parent).len() == 1 {
        parent.parent().unwrap_or(parent)
    } else {
        parent
    }
}

fn whole_lines(c: &Context<'_>, n: Node<'_>) -> (usize, usize) {
    (
        c.source
            .line_start(n.start_position().row + 1)
            .unwrap_or(n.start_byte()),
        c.source
            .line_start(n.end_position().row + 2)
            .unwrap_or(c.source.as_bytes().len()),
    )
}
fn kernel_receiver(c: &Context<'_>, n: Node<'_>) -> bool {
    field(n, "receiver").is_none_or(|r| {
        matches!(r.kind(), "constant" | "scope_resolution")
            && c.text(r).rsplit("::").next() == Some("Kernel")
    })
}
fn same_requirement(c: &Context<'_>, a: Node<'_>, b: Node<'_>) -> bool {
    require_call(c, a)
        && c.method(a) == c.method(b)
        && canonical(args(a)[0], c) == canonical(args(b)[0], c)
}
