use super::super::context::*;
use tree_sitter::Node;

const CONSTANT: &str = "Stub constant instead of declaring explicitly.";

pub(super) fn spec_constants(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if super::super::custom_index::in_spec(c, n) {
        return vec![];
    }
    match n.kind() {
        "class" => vec![Finding::new(
            n,
            "Stub class constant instead of declaring explicitly.",
        )],
        "module" => vec![Finding::new(
            n,
            "Stub module constant instead of declaring explicitly.",
        )],
        _ => assignment_targets(n)
            .into_iter()
            .map(|target| Finding::new(target, CONSTANT))
            .collect(),
    }
}

fn assignment_targets(n: Node<'_>) -> Vec<Node<'_>> {
    let Some(left) = field(n, "left").or_else(|| field(n, "pattern")) else {
        return vec![];
    };
    if n.kind() == "assignment" && constant(left) {
        return vec![n];
    }
    constant_targets(left)
}

fn constant_targets(n: Node<'_>) -> Vec<Node<'_>> {
    if constant(n) {
        return vec![n];
    }
    match n.kind() {
        "left_assignment_list" | "destructured_left_assignment" | "rest_assignment" => {
            kids(n).into_iter().flat_map(constant_targets).collect()
        }
        _ => vec![],
    }
}

fn constant(n: Node<'_>) -> bool {
    matches!(n.kind(), "constant" | "scope_resolution")
}
