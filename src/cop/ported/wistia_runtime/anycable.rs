use super::super::context::*;
use tree_sitter::Node;

pub(super) fn instance_vars(_c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if ancestors(n).iter().any(|p| p.kind() == "class") {
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
    if callback(c, n) {
        return vec![Finding::new(
            n,
            "Custom stream callbacks are not supported in AnyCable",
        )];
    }
    if !valid_stream_arguments(c, n) {
        return vec![];
    }
    descendants(n)
        .into_iter()
        .filter(|p| custom_coder(c, *p))
        .map(|p| Finding::new(p, "Custom coders are not supported in AnyCable"))
        .collect()
}

fn callback(c: &Context<'_>, n: Node<'_>) -> bool {
    block(n).is_some()
        || args(n).iter().any(|v| {
            v.kind() == "lambda" || is_call(*v) && c.method(*v) == "lambda" && block(*v).is_some()
        })
}

fn custom_coder(c: &Context<'_>, n: Node<'_>) -> bool {
    n.kind() == "pair"
        && coder_key(c, n)
        && n.child_by_field_name("value")
            .is_none_or(|v| c.text(v) != "ActiveSupport::JSON")
}

fn coder_key(c: &Context<'_>, n: Node<'_>) -> bool {
    n.child_by_field_name("key")
        .is_some_and(|k| c.value(k) == "coder")
}

fn valid_stream_arguments(c: &Context<'_>, n: Node<'_>) -> bool {
    c.method(n) != "stream_from" || args(n).first().is_some_and(|a| a.kind() == "string")
}
