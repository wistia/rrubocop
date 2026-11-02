use super::super::context::*;
use tree_sitter::Node;
pub(super) fn regexp_condition(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let mut expr = n;
    let mut manual = false;
    while let Some(parent) = expr.parent() {
        match parent.kind() {
            "parenthesized_statements" => expr = parent,
            "unary" if boolean_operator(c, parent) => {
                manual |= operator(c, parent) == "!";
                expr = parent;
            }
            "binary" if ["&&", "||", "and", "or"].contains(&operator(c, parent)) => {
                expr = parent;
            }
            _ if condition_parent(parent, expr) => return vec![finding(c, n, manual)],
            _ => return vec![],
        }
    }
    vec![]
}
fn boolean_operator(c: &Context<'_>, n: Node<'_>) -> bool {
    ["!", "not"].contains(&operator(c, n))
}
fn condition_parent(parent: Node<'_>, expression: Node<'_>) -> bool {
    matches!(
        parent.kind(),
        "if" | "unless"
            | "if_modifier"
            | "unless_modifier"
            | "while"
            | "until"
            | "while_modifier"
            | "until_modifier"
            | "conditional"
    ) && field(parent, "condition") == Some(expression)
}
fn finding(c: &Context<'_>, n: Node<'_>, unary: bool) -> Finding {
    let finding = Finding::new(
        n,
        "Do not use regexp literal as a condition. The regexp literal matches `$_` implicitly.",
    );
    if unary {
        finding
    } else {
        finding.replace_with(c, n, || format!("{} =~ $_", c.text(n)))
    }
}
