use super::super::context::*;
use tree_sitter::Node;

pub(super) fn migration(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if !migration_class(c, n) {
        return vec![];
    }
    let methods = migration_methods(c, n);
    if methods.contains(&"change") || methods.contains(&"up") && methods.contains(&"down") {
        return vec![];
    }
    vec![Finding::new(
        n,
        "Migrations must contain either a `change` method, or both an `up` and a `down` method.",
    )]
}

fn migration_methods<'a>(c: &Context<'a>, n: Node<'_>) -> Vec<&'a str> {
    descendants(n)
        .filter(|d| empty_method(*d))
        .map(|d| c.method(d))
        .collect()
}

fn empty_method(n: Node<'_>) -> bool {
    n.kind() == "method" && field(n, "parameters").is_none_or(|p| kids(p).is_empty())
}

pub(in crate::cop::ported) fn migration_class(c: &Context<'_>, n: Node<'_>) -> bool {
    let Some(name) = field(n, "name") else {
        return false;
    };
    if name.kind() != "constant"
        && !(name.kind() == "scope_resolution" && field(name, "scope").is_none())
    {
        return false;
    }
    field(n, "superclass")
        .and_then(|parent| parent.named_child(0))
        .is_some_and(|parent| migration_parent(c, parent))
}

fn migration_parent(c: &Context<'_>, n: Node<'_>) -> bool {
    let Some((receiver, arguments)) = receiver_and_versions(c, n) else {
        return false;
    };
    c.constant_name(receiver).as_deref() == Some("ActiveRecord::Migration")
        && matches!(arguments.as_slice(), [version] if float_version(c, *version))
}

fn receiver_and_versions<'n>(c: &Context<'_>, n: Node<'n>) -> Option<(Node<'n>, Vec<Node<'n>>)> {
    if n.kind() == "element_reference" {
        let children = kids(n);
        Some((*children.first()?, children[1..].to_vec()))
    } else if version_call(c, n) {
        Some((
            field(n, "receiver")?,
            field(n, "arguments").map(kids).unwrap_or_default(),
        ))
    } else {
        None
    }
}

fn float_version(c: &Context<'_>, n: Node<'_>) -> bool {
    n.kind() == "float"
        || n.kind() == "unary"
            && matches!(operator(c, n), "+" | "-")
            && matches!(kids(n).as_slice(), [number] if number.kind() == "float")
}

fn version_call(c: &Context<'_>, n: Node<'_>) -> bool {
    is_call(n) && !safe(c, n) && block(n).is_none() && c.method(n) == "[]"
}
