use super::super::context::*;
use super::policies::superclass;
use tree_sitter::Node;

pub(super) fn migration(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let parent = superclass(c, n);
    if !parent.starts_with("ActiveRecord::Migration")
        && !parent.starts_with("::ActiveRecord::Migration")
    {
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
