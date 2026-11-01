use super::super::context::*;
use super::branches;
use tree_sitter::Node;
pub(super) fn or_assignment(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let pair = if n.kind() == "assignment" {
        ternary_parts(c, n)
    } else {
        unless_parts(c, n)
    };
    let Some((var, value)) = pair else {
        return vec![];
    };
    let finding = Finding::new(n, "Use the double pipe equals operator `||=` instead.");
    vec![if descendants(n).any(|node| node.kind() == "comment") {
        finding
    } else {
        finding.replace_with(c, n, || format!("{} ||= {}", c.text(var), c.text(value)))
    }]
}
fn ternary_parts<'n>(c: &Context<'_>, n: Node<'n>) -> Option<(Node<'n>, Node<'n>)> {
    let left = field(n, "left")?;
    let cond = field(n, "right")?;
    if !variable(left) {
        return None;
    }
    ternary_default(c, left, cond).map(|value| (left, value))
}
fn variable(n: Node<'_>) -> bool {
    matches!(
        n.kind(),
        "identifier" | "instance_variable" | "class_variable" | "global_variable"
    )
}
fn ternary_default<'n>(c: &Context<'_>, left: Node<'n>, cond: Node<'n>) -> Option<Node<'n>> {
    let (yes, no) = branches(cond)?;
    let test = field(cond, "condition")?;
    let yes = yes?;
    let no = no?;
    (c.text(left) == c.text(test)
        && c.text(left) == c.text(yes)
        && !matches!(no.kind(), "if" | "unless" | "conditional" | "elsif"))
    .then_some(no)
}
fn unless_parts<'n>(c: &Context<'_>, n: Node<'n>) -> Option<(Node<'n>, Node<'n>)> {
    let assignment = unless_assignment(n)?;
    let test = field(n, "condition")?;
    let left = field(assignment, "left")?;
    if !variable(left) || !bound_test(c, test) || c.text(test) != c.text(left) {
        return None;
    }
    Some((left, field(assignment, "right")?))
}
fn unless_assignment(n: Node<'_>) -> Option<Node<'_>> {
    let (yes, no) = branches(n)?;
    if !single_arm(n) || yes.is_some() {
        return None;
    }
    no.filter(|n| n.kind() == "assignment")
}
fn single_arm(n: Node<'_>) -> bool {
    field(n, "consequence")
        .or_else(|| field(n, "body"))
        .or_else(|| field(n, "alternative"))
        .is_none_or(|a| statements(a).len() == 1)
}

fn bound_test(c: &Context<'_>, test: Node<'_>) -> bool {
    test.kind() != "identifier" || local_identifier(c, test)
}
