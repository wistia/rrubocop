use super::super::context::*;
use tree_sitter::Node;
mod inflections;
mod location;
mod names;
const MESSAGE: &str =
    "This constant is undiscoverable by the auto loader due to where it is nested.";
pub(super) fn check(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if !definition(n) {
        return vec![];
    }
    let Some((root, relative)) = location::resolve(&c.source.path) else {
        return vec![];
    };
    let acronyms = inflections::acronyms(&root);
    let expected = expected_name(&relative, &acronyms);
    let Some(actual) = names::defined(c, n, &acronyms) else {
        return vec![];
    };
    if compatible(
        &expected,
        actual.strip_prefix("Object::").unwrap_or(&actual),
    ) {
        vec![]
    } else {
        vec![Finding::new(n, MESSAGE)]
    }
}
fn compatible(expected: &str, actual: &str) -> bool {
    let expected = expected.split("::").collect::<Vec<_>>();
    let actual = actual.split("::").collect::<Vec<_>>();
    expected.starts_with(&actual) || actual.starts_with(&expected)
}

fn expected_name(relative: &str, acronyms: &[String]) -> String {
    inflections::camelize(&relative.replace(".rb", ""), acronyms)
}
fn definition(n: Node<'_>) -> bool {
    match n.kind() {
        "class" | "module" => true,
        "assignment" | "operator_assignment" => field(n, "left").is_some_and(constant),
        "constant" | "scope_resolution" => n.parent().is_some_and(|p| {
            matches!(
                p.kind(),
                "left_assignment_list" | "destructured_left_assignment"
            )
        }),
        _ => false,
    }
}
fn constant(n: Node<'_>) -> bool {
    matches!(n.kind(), "constant" | "scope_resolution")
}
