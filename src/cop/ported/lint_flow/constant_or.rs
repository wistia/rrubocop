use super::super::context::*;
use tree_sitter::Node;
pub(super) fn constant_or(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if operator(c, n) != "||=" || !constant_target(n) {
        return vec![];
    }
    let Some(op) = field(n, "operator") else {
        return vec![];
    };
    let finding = Finding::new(op, "Avoid using or-assignment with constants.");
    if inside_method(n) {
        vec![finding]
    } else {
        vec![finding.replace(op, "=")]
    }
}
fn constant_target(n: Node<'_>) -> bool {
    field(n, "left").is_some_and(|l| matches!(l.kind(), "constant" | "scope_resolution"))
}
fn inside_method(n: Node<'_>) -> bool {
    ancestors(n)
        .iter()
        .any(|p| matches!(p.kind(), "method" | "singleton_method"))
}
