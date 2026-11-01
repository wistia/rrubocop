use super::super::context::*;
use tree_sitter::Node;
pub(super) fn redundant_assignment(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let findings = body(n).map(|b| redundant_branch(c, b)).unwrap_or_default();
    if !findings.is_empty() && observers(c, n) {
        findings.into_iter().map(manual).collect()
    } else {
        findings
    }
}
fn redundant_branch(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if matches!(n.kind(), "rescue" | "in_clause" | "when") {
        return body(n).map(|b| redundant_branch(c, b)).unwrap_or_default();
    }
    if let Some(ensure) = kids(n).into_iter().find(|k| k.kind() == "ensure") {
        return redundant_sequence(c, &kids(ensure));
    }
    let sequence = if n.kind() == "begin" {
        kids(n)
    } else {
        statements(n)
    };
    sequence_branches(c, &sequence)
}
fn sequence_branches(c: &Context<'_>, sequence: &[Node<'_>]) -> Vec<Finding> {
    let split = sequence
        .iter()
        .position(|n| matches!(n.kind(), "rescue" | "else" | "ensure"))
        .unwrap_or(sequence.len());
    let mut out = redundant_sequence(c, &sequence[..split]);
    for handler in &sequence[split..] {
        if handler.kind() != "ensure" {
            out.extend(redundant_branch(c, *handler));
        }
    }
    out
}
fn redundant_sequence(c: &Context<'_>, s: &[Node<'_>]) -> Vec<Finding> {
    if let Some(finding) = last_assignment(c, s) {
        return vec![finding];
    }
    s.last()
        .map(|last| descend_branch(c, *last))
        .unwrap_or_default()
}

fn last_assignment(c: &Context<'_>, s: &[Node<'_>]) -> Option<Finding> {
    if s.len() < 2 {
        return None;
    }
    let assignment = s[s.len() - 2];
    let returned = s[s.len() - 1];
    if !same_return(c, assignment, returned) {
        return None;
    }
    let right = field(assignment, "right")?;
    Some(
        Finding::new(
            assignment,
            "Redundant assignment before returning detected.",
        )
        .edit_with(c, assignment.start_byte(), returned.end_byte(), || {
            format!(
                "{}{}",
                c.text(right),
                String::from_utf8_lossy(
                    &c.source.as_bytes()[assignment.end_byte()..returned.start_byte()]
                )
            )
        }),
    )
}
fn same_return(c: &Context<'_>, assignment: Node<'_>, returned: Node<'_>) -> bool {
    assignment.kind() == "assignment"
        && returned.kind() == "identifier"
        && field(assignment, "left")
            .is_some_and(|l| l.kind() == "identifier" && c.text(l) == c.text(returned))
}
fn descend_branch(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    match n.kind() {
        "if" | "unless" | "elsif" => condition_branches(c, n),
        "case" | "case_match" => case_branches(c, n),
        "body_statement" | "begin" | "rescue" | "else" => redundant_branch(c, n),
        _ => vec![],
    }
}
fn case_branches(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    kids(n)
        .into_iter()
        .filter(|k| ["when", "in_clause", "else"].contains(&k.kind()))
        .flat_map(|b| redundant_branch(c, b))
        .collect()
}

fn condition_branches(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    kids(n)
        .into_iter()
        .filter(|k| Some(*k) != field(n, "condition"))
        .flat_map(|k| redundant_branch(c, k))
        .collect()
}

fn observers(c: &Context<'_>, n: Node<'_>) -> bool {
    descendants(n).any(|node| {
        matches!(node.kind(), "block" | "do_block" | "lambda")
            || (is_call(node) || node.kind() == "identifier")
                && matches!(c.method(node), "binding" | "eval" | "caller_binding")
    })
}
fn manual(mut finding: Finding) -> Finding {
    finding.correctable = false;
    finding.edit = None;
    finding.additional.clear();
    finding
}
