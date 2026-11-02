use super::super::context::*;
use tree_sitter::Node;

pub(super) fn capitalized(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if c.method(n) != "attribute" {
        return vec![];
    }
    let value = if let Some(pair) = attribute_key(c, n) {
        field(pair, "value")
    } else {
        args(n)
            .into_iter()
            .find(|argument| argument.kind() != "comment")
    };
    let Some(value) = value else { return vec![] };
    if !capitalized_literal(c, value) {
        return vec![];
    }
    vec![Finding::new(
        value,
        "Prefer keys that are lowercase or snake_case.",
    )]
}

fn capitalized_literal(c: &Context<'_>, n: Node<'_>) -> bool {
    c.static_symbol(n)
        .or_else(|| c.ruby_static_string(n))
        .is_some_and(|value| {
            if value.is_ascii() {
                value.bytes().any(|b| b.is_ascii_uppercase())
            } else {
                value.to_lowercase() != value
            }
        })
}

pub(super) fn key_block(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if c.method(n) == "attribute" && block(n).is_some() && attribute_key(c, n).is_some() {
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
    let Some(attribute) = args(n)
        .into_iter()
        .find(|argument| argument.kind() != "comment")
        .filter(|a| c.symbol(*a))
    else {
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
    c.static_symbol(attribute)
        .is_some_and(|name| super::super::custom_index::index(c, n).method(c, n, &name, false))
}

fn attribute_block(c: &Context<'_>, n: Node<'_>) -> bool {
    c.method(n) == "attribute" && block(n).is_some() && c.receiver(n).is_empty()
}

fn attribute_key<'n>(c: &Context<'_>, n: Node<'n>) -> Option<Node<'n>> {
    let arguments = args(n)
        .into_iter()
        .filter(|argument| argument.kind() != "comment")
        .collect::<Vec<_>>();
    let last = *arguments.last()?;
    if last.kind() == "hash" {
        return option_pairs(last).find(|pair| key_pair(c, *pair));
    }
    implicit_key(c, &arguments)
}
fn implicit_key<'n>(c: &Context<'_>, arguments: &[Node<'n>]) -> Option<Node<'n>> {
    arguments
        .iter()
        .rev()
        .copied()
        .take_while(|argument| implicit_option(*argument))
        .filter(|pair| key_pair(c, *pair))
        .last()
}
fn implicit_option(n: Node<'_>) -> bool {
    matches!(n.kind(), "pair" | "hash_splat_argument")
}
fn key_pair(c: &Context<'_>, n: Node<'_>) -> bool {
    field(n, "key").is_some_and(|key| c.static_symbol(key).as_deref() == Some("key"))
}
