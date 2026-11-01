use super::super::context::*;
use tree_sitter::Node;

pub(super) fn capitalized(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if c.method(n) != "attribute" {
        return vec![];
    }
    let Some(value) = c
        .symbol_pair_value(n, "key")
        .or_else(|| args(n).first().copied())
    else {
        return vec![];
    };
    if !capitalized_literal(c, value) {
        return vec![];
    }
    vec![Finding::new(
        value,
        "Prefer keys that are lowercase or snake_case.",
    )]
}

fn capitalized_literal(c: &Context<'_>, n: Node<'_>) -> bool {
    (c.symbol(n) || n.kind() == "string") && {
        let value = c.value(n);
        if value.is_ascii() {
            value.bytes().any(|b| b.is_ascii_uppercase())
        } else {
            value.to_lowercase() != value
        }
    }
}

pub(super) fn key_block(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if c.method(n) == "attribute" && block(n).is_some() && c.symbol_pair_value(n, "key").is_some() {
        vec![Finding::new(
            n,
            "Combining key and block could indicate a code error. The attribute field is ignored in this case.",
        )]
    } else {
        vec![]
    }
}

pub(super) fn method_block(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if !attribute_block(c, n) {
        return vec![];
    }
    let Some(attribute) = args(n).first().copied().filter(|a| c.symbol(*a)) else {
        return vec![];
    };
    if !matching_method(c, n, attribute) {
        return vec![];
    }
    vec![Finding::new(
        n,
        "Combining method and block could indicate a code error. The method is ignored in this case.",
    )]
}

fn matching_method(c: &Context<'_>, n: Node<'_>, attribute: Node<'_>) -> bool {
    super::super::custom_index::index(c, n).method(n, c.value(attribute), false)
}

fn attribute_block(c: &Context<'_>, n: Node<'_>) -> bool {
    c.method(n) == "attribute" && block(n).is_some() && c.receiver(n).is_empty()
}
