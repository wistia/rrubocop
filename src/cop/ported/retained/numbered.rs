use super::super::context::*;
use tree_sitter::Node;
pub(super) fn check(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let Some(digits) = digits(c.text(n)) else {
        return vec![];
    };
    let Some(target) = assignment(n) else {
        return vec![];
    };
    let number = digits.trim_start_matches('0');
    let number = if number.is_empty() { "0" } else { number };
    let description = if number.len() == 1 && number != "0" {
        "is reserved for numbered parameter"
    } else {
        "is similar to numbered parameter"
    };
    vec![Finding::new(
        target,
        format!("`_{number}` {description}; consider another name."),
    )]
}
fn assignment(n: Node<'_>) -> Option<Node<'_>> {
    let p = n.parent()?;
    match p.kind() {
        "assignment" if field(p, "left") == Some(n) => Some(p),
        "operator_assignment" if field(p, "left") == Some(n) => Some(n),
        "left_assignment_list"
        | "destructured_left_assignment"
        | "rest_assignment"
        | "exception_variable" => Some(n),
        "for" if field(p, "pattern") == Some(n) => Some(n),
        _ => None,
    }
}

fn digits(name: &str) -> Option<&str> {
    name.strip_prefix('_')
        .filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
}
