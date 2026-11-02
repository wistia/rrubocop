use super::super::context::*;
use std::collections::HashMap;
use tree_sitter::Node;

pub(super) fn safe_consistency(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if !logical(c, n) || logical_parent(c, n) {
        return vec![];
    }
    let conditions = logical_conditions(c, n);
    if !conditions
        .iter()
        .any(|condition| unsafe_call(c, *condition))
    {
        return vec![];
    }
    consistency_findings(c, &conditions, &safe_calls(c, &conditions))
}
fn logical(c: &Context<'_>, n: Node<'_>) -> bool {
    n.kind() == "binary" && ["&&", "||", "and", "or"].contains(&operator(c, n))
}
fn logical_parent(c: &Context<'_>, n: Node<'_>) -> bool {
    let Some(parent) = n.parent() else {
        return false;
    };
    if parent.kind() == "parenthesized_statements" {
        return parent.parent().is_some_and(|p| logical(c, p));
    }
    logical(c, parent)
}
fn safe_calls<'a>(c: &Context<'_>, conditions: &[Node<'a>]) -> HashMap<Option<String>, Node<'a>> {
    let mut calls = HashMap::new();
    for condition in conditions.iter().filter(|d| safe_expression(c, **d)) {
        calls.entry(receiver(c, *condition)).or_insert(*condition);
    }
    calls
}
fn safe_expression(c: &Context<'_>, n: Node<'_>) -> bool {
    call_expression(n).is_some_and(|call| safe(c, call))
}
fn call_expression(n: Node<'_>) -> Option<Node<'_>> {
    match n.kind() {
        "assignment" => field(n, "left").filter(|n| is_call(*n)),
        "binary" | "element_reference" | "call" => Some(n),
        _ => None,
    }
}
fn receiver(c: &Context<'_>, n: Node<'_>) -> Option<String> {
    let call = call_expression(n)?;
    let field_name = match call.kind() {
        "binary" => "left",
        "element_reference" => "object",
        _ => "receiver",
    };
    field(call, field_name)
        .map(|receiver| super::super::lint_duplicates::expression_key(c, receiver))
}
fn expression_method<'a>(c: &'a Context<'_>, n: Node<'_>) -> &'a str {
    let Some(call) = call_expression(n) else {
        return "";
    };
    if call.kind() == "binary" {
        operator(c, call)
    } else {
        c.method(call)
    }
}
fn unsafe_call(c: &Context<'_>, n: Node<'_>) -> bool {
    call_expression(n).is_some()
        && !safe_expression(c, n)
        && !nil_method(c, expression_method(c, n))
}
fn consistency_findings(
    c: &Context<'_>,
    conditions: &[Node<'_>],
    safe_calls: &HashMap<Option<String>, Node<'_>>,
) -> Vec<Finding> {
    if safe_calls.is_empty() {
        return vec![];
    }
    conditions
        .iter()
        .filter(|d| unsafe_call(c, **d))
        .filter_map(|d| consistency_finding(c, *d, safe_calls))
        .collect()
}
fn consistency_finding(
    c: &Context<'_>,
    n: Node<'_>,
    safe_calls: &HashMap<Option<String>, Node<'_>>,
) -> Option<Finding> {
    let original = safe_calls.get(&receiver(c, n))?;
    let (start, end) = joined_range(*original, n);
    Some(with_safe_navigation(
        c,
        Finding::range(
            start,
            end,
            "Ensure that safe navigation is used consistently inside of `&&` and `||`.",
        ),
        n,
    ))
}

fn with_safe_navigation(c: &Context<'_>, finding: Finding, n: Node<'_>) -> Finding {
    let dot = call_expression(n)
        .and_then(|call| field(call, "operator"))
        .filter(|dot| c.text(*dot) == ".");
    match dot {
        Some(dot) => finding.edit_with(c, dot.start_byte(), dot.start_byte(), || "&"),
        None => finding,
    }
}
fn joined_range(a: Node<'_>, b: Node<'_>) -> (usize, usize) {
    (
        a.start_byte().min(b.start_byte()),
        a.end_byte().max(b.end_byte()),
    )
}
fn logical_conditions<'a>(c: &Context<'_>, n: Node<'a>) -> Vec<Node<'a>> {
    if n.kind() == "parenthesized_statements" || logical(c, n) {
        kids(n)
            .into_iter()
            .flat_map(|k| logical_conditions(c, k))
            .collect()
    } else {
        vec![n]
    }
}
fn nil_method(c: &Context<'_>, method: &str) -> bool {
    if [
        "nil?",
        "is_a?",
        "kind_of?",
        "respond_to?",
        "to_a",
        "to_s",
        "to_i",
        "to_f",
        "to_h",
        "inspect",
        "class",
        "object_id",
        "!",
        "==",
        "!=",
        "equal?",
        "eql?",
        "hash",
        "freeze",
        "frozen?",
    ]
    .contains(&method)
    {
        return true;
    }
    c.config
        .options
        .get("AllowedMethods")
        .and_then(|v| v.as_sequence())
        .is_some_and(|v| v.iter().any(|s| s.as_str() == Some(method)))
}
