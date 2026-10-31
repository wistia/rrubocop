mod binding;
mod index;
use super::super::context::*;
use tree_sitter::Node;

pub(super) fn self_assignment(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let matches = if is_call(n) {
        setter_call(c, n)
    } else {
        assignment(c, n)
    };
    if matches {
        vec![Finding::new(n, "Self-assignment detected.")]
    } else {
        vec![]
    }
}
fn assignment(c: &Context<'_>, n: Node<'_>) -> bool {
    if n.kind() == "operator_assignment" && !["||=", "&&="].contains(&operator(c, n)) {
        return false;
    }
    let Some((left, right)) = field(n, "left").zip(field(n, "right")) else {
        return false;
    };
    if n.kind() == "operator_assignment" {
        return variable_target(left) && same_assignment(c, left, right);
    }
    same_assignment(c, left, right)
}
fn same_assignment(c: &Context<'_>, left: Node<'_>, right: Node<'_>) -> bool {
    if left.kind() == "left_assignment_list" {
        return multiple_assignment(c, left, right);
    }
    if left.kind() == "element_reference" {
        return index::assignment(c, left, right);
    }
    if canonical(left, c) != canonical(right, c) {
        return false;
    }
    scalar_assignment(left)
}
fn multiple_assignment(c: &Context<'_>, left: Node<'_>, right: Node<'_>) -> bool {
    if !matches!(right.kind(), "right_assignment_list" | "array") {
        return false;
    }
    let l = kids(left);
    let r = kids(right);
    l.len() == r.len()
        && l.iter()
            .zip(r)
            .all(|(l, r)| variable_target(*l) && same_assignment(c, *l, r))
}
fn scalar_assignment(left: Node<'_>) -> bool {
    if is_call(left) {
        return args(left).is_empty();
    }
    variable_target(left) || matches!(left.kind(), "constant" | "scope_resolution")
}
fn variable_target(n: Node<'_>) -> bool {
    matches!(
        n.kind(),
        "identifier" | "instance_variable" | "class_variable" | "global_variable"
    )
}
fn setter_call(c: &Context<'_>, n: Node<'_>) -> bool {
    let arguments = args(n);
    if c.method(n) == "[]=" {
        return index::setter(c, n, &arguments);
    }
    if !c.method(n).ends_with('=') || arguments.len() != 1 {
        return false;
    }
    attribute_setter(c, n, arguments[0])
}
fn attribute_setter(c: &Context<'_>, n: Node<'_>, right: Node<'_>) -> bool {
    is_call(right)
        && args(right).is_empty()
        && c.receiver(n) == c.receiver(right)
        && c.method(right) == c.method(n).trim_end_matches('=')
}
