use super::super::context::*;
use tree_sitter::Node;

pub(super) fn respond_to(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if c.method(n) != "respond_to" {
        return vec![];
    }
    let Some(block) = block(n) else {
        return vec![];
    };
    let Some(parameter) = single_parameter(block) else {
        return vec![];
    };
    let Some(body) = body(block) else {
        return vec![];
    };
    if has_any(c, body, parameter) {
        return vec![];
    }
    vec![Finding::new(
        n,
        "Every `respond_to` block must include a `format.any`.",
    )]
}

fn single_parameter(n: Node<'_>) -> Option<Node<'_>> {
    let parameters = n
        .child_by_field_name("parameters")
        .map(kids)
        .unwrap_or_default();
    (parameters.len() == 1).then(|| parameters[0])
}

fn has_any(c: &Context<'_>, body: Node<'_>, parameter: Node<'_>) -> bool {
    statements(body)
        .iter()
        .any(|call| c.method(*call) == "any" && c.receiver(*call) == c.text(parameter))
}
