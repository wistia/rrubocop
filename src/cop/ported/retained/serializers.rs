use super::super::context::*;
use super::arguments as args;
use tree_sitter::Node;

pub(super) fn single(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if c.method(n) != "attribute" || block(n).is_some() {
        return vec![];
    }
    let Some(attribute) = args(n).first().copied() else {
        return vec![];
    };
    if matching_method(c, n, attribute).is_none() {
        return vec![];
    }
    vec![Finding::new(n, "Prefer block attribute over method call.")]
}
pub(super) fn plural(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if c.method(n) != "attributes" || block(n).is_some() {
        return vec![];
    }
    args(n)
        .into_iter()
        .rev()
        .filter_map(|attribute| {
            matching_method(c, n, attribute)?;
            Some(Finding::new(
                attribute,
                "Prefer block attribute over method call from attributes.",
            ))
        })
        .collect()
}

fn scope(n: Node<'_>) -> Option<Node<'_>> {
    ancestors(n).find(|p| matches!(p.kind(), "class" | "module" | "singleton_class"))
}
fn matching_method<'n>(c: &Context<'_>, n: Node<'n>, attribute: Node<'_>) -> Option<Node<'n>> {
    if !c.symbol(attribute) {
        return None;
    }
    let owner = scope(n)?;
    descendants(owner).find(|m| {
        m.kind() == "method" && c.method(*m) == c.value(attribute) && scope(*m) == Some(owner)
    })
}

#[cfg(test)]
mod tests;
