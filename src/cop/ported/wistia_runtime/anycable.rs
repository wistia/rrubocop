use super::super::context::*;
use tree_sitter::Node;

pub(super) fn instance_vars(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if super::super::custom_index::in_class(c, n) {
        vec![Finding::new(
            n,
            "Channel instance variables are not supported in AnyCable. Use `state_attr_accessor` instead",
        )]
    } else {
        vec![]
    }
}

pub(super) fn stream(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if !matches!(c.method(n), "stream_from" | "stream_for") {
        return vec![];
    }
    let values = args(n);
    if callback(c, n, &values) {
        return vec![Finding::new(
            n,
            "Custom stream callbacks are not supported in AnyCable",
        )];
    }
    if !valid_stream_arguments(c, n, &values) {
        return vec![];
    }
    coder_findings(c, values)
}
fn coder_findings(c: &Context<'_>, values: Vec<Node<'_>>) -> Vec<Finding> {
    values
        .into_iter()
        .filter(|n| matches!(n.kind(), "pair" | "hash"))
        .flat_map(option_pairs)
        .filter(|p| custom_coder(c, *p))
        .map(|p| Finding::new(p, "Custom coders are not supported in AnyCable"))
        .collect()
}

fn callback(c: &Context<'_>, n: Node<'_>, values: &[Node<'_>]) -> bool {
    block(n).is_some()
        || values.iter().any(|v| {
            v.kind() == "lambda" || is_call(*v) && c.method(*v) == "lambda" && block(*v).is_some()
        })
}

fn custom_coder(c: &Context<'_>, n: Node<'_>) -> bool {
    n.kind() == "pair"
        && coder_key(c, n)
        && n.child_by_field_name("value")
            .is_none_or(|v| !active_support_json(c, v))
}

fn coder_key(c: &Context<'_>, n: Node<'_>) -> bool {
    n.child_by_field_name("key")
        .is_some_and(|k| c.symbol(k) && c.value(k) == "coder")
}

fn valid_stream_arguments(c: &Context<'_>, n: Node<'_>, values: &[Node<'_>]) -> bool {
    c.method(n) != "stream_from"
        || values
            .iter()
            .find(|a| a.kind() != "comment")
            .is_some_and(|a| a.kind() == "string")
}

fn active_support_json(c: &Context<'_>, n: Node<'_>) -> bool {
    n.kind() == "scope_resolution"
        && field(n, "scope")
            .is_some_and(|scope| scope.kind() == "constant" && c.text(scope) == "ActiveSupport")
        && field(n, "name").is_some_and(|name| c.text(name) == "JSON")
}
