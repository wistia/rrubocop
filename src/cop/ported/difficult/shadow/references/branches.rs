use super::*;

pub(super) fn exclusive(a: Node<'_>, b: Node<'_>) -> bool {
    let a = branches(a);
    branches(b).iter().any(|(control, child)| {
        a.iter()
            .any(|(other, branch)| other == control && branch != child)
    })
}
fn branches(mut n: Node<'_>) -> Vec<(usize, usize)> {
    let mut result = vec![];
    while let Some(parent) = n.parent() {
        if conditional_branch(parent, n) {
            result.push((parent.id(), n.id()));
        }
        n = parent;
    }
    result
}
fn conditional_branch(parent: Node<'_>, child: Node<'_>) -> bool {
    match parent.kind() {
        "if" | "unless" | "elsif" | "conditional" => field(parent, "condition") != Some(child),
        "case" | "case_match" => field(parent, "value") != Some(child),
        _ => false,
    }
}
