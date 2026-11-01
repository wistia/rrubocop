use super::super::context::*;
use tree_sitter::Node;

pub(super) fn spec_constants(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let Some(message) = constant_message(n) else {
        return vec![];
    };
    if super::super::custom_index::in_spec(c, n) {
        return vec![];
    }
    vec![Finding::new(n, message)]
}

fn constant_message(n: Node<'_>) -> Option<&'static str> {
    match n.kind() {
        "class" => Some("Stub class constant instead of declaring explicitly."),
        "module" => Some("Stub module constant instead of declaring explicitly."),
        "assignment"
            if n.child_by_field_name("left")
                .is_some_and(|l| matches!(l.kind(), "constant" | "scope_resolution")) =>
        {
            Some("Stub constant instead of declaring explicitly.")
        }
        _ => None,
    }
}
