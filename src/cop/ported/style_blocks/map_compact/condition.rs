use super::super::super::context::*;
use tree_sitter::Node;
pub(super) fn conditional_map<'n>(
    c: &Context<'_>,
    b: Node<'n>,
    arg: &str,
) -> Option<(Node<'n>, bool)> {
    let s = statements(b);
    let cond = *s.first()?;
    if !matches!(
        cond.kind(),
        "if" | "unless" | "if_modifier" | "unless_modifier" | "conditional"
    ) {
        return None;
    }
    let (yes, no) = map_branches(cond)?;
    let positive = if s.len() == 1 {
        branch_result(c, yes, no, arg)
    } else {
        guard_result(c, &s, yes, no, arg)
    }?;
    Some((field(cond, "condition")?, positive))
}
fn map_branches(n: Node<'_>) -> Option<(Option<Node<'_>>, Option<Node<'_>>)> {
    let yes = single_branch(field(n, "consequence").or_else(|| field(n, "body")))?;
    let no = single_branch(field(n, "alternative"))?;
    Some(if n.kind().starts_with("unless") {
        (no, yes)
    } else {
        (yes, no)
    })
}
fn single_branch(branch: Option<Node<'_>>) -> Option<Option<Node<'_>>> {
    let Some(branch) = branch else {
        return Some(None);
    };
    let nodes = statements(branch);
    match nodes.as_slice() {
        [] => Some(None),
        [node] => Some(Some(*node)),
        _ => None,
    }
}

fn branch_result(
    c: &Context<'_>,
    yes: Option<Node<'_>>,
    no: Option<Node<'_>>,
    arg: &str,
) -> Option<bool> {
    if returns_arg(c, yes, arg)
        && (empty_branch(c, no) || next_arg(c, yes, arg) && nil_branch(c, no))
    {
        return Some(true);
    }
    if returns_arg(c, no, arg)
        && (empty_branch(c, yes) || next_arg(c, no, arg) && nil_branch(c, yes))
    {
        return Some(false);
    }
    None
}
fn returns_arg(c: &Context<'_>, branch: Option<Node<'_>>, arg: &str) -> bool {
    branch
        .and_then(|b| {
            let s = statements(b);
            (s.len() == 1).then(|| s[0])
        })
        .is_some_and(|v| c.text(v) == arg || next_value(c, v, arg))
}
fn empty_branch(c: &Context<'_>, branch: Option<Node<'_>>) -> bool {
    branch.is_none_or(|b| {
        let k = statements(b);
        k.len() == 1 && c.text(k[0]) == "next"
    })
}
fn guard_result(
    c: &Context<'_>,
    s: &[Node<'_>],
    yes: Option<Node<'_>>,
    no: Option<Node<'_>>,
    arg: &str,
) -> Option<bool> {
    if let Some(result) = nil_guard(c, s, yes, no, arg) {
        return Some(result);
    }
    if s.len() != 2 || c.text(s[1]) != arg {
        return None;
    }
    trailing_guard(yes, no)
}
fn nil_guard(
    c: &Context<'_>,
    s: &[Node<'_>],
    yes: Option<Node<'_>>,
    no: Option<Node<'_>>,
    arg: &str,
) -> Option<bool> {
    if s.len() != 2 || c.text(s[1]) != "nil" {
        return None;
    }
    if no.is_none() && next_arg(c, yes, arg) {
        return Some(true);
    }
    if yes.is_none() && next_arg(c, no, arg) {
        return Some(false);
    }
    None
}

fn next_arg(c: &Context<'_>, branch: Option<Node<'_>>, arg: &str) -> bool {
    branch
        .and_then(|b| statements(b).first().copied())
        .is_some_and(|v| next_value(c, v, arg))
}
fn nil_branch(c: &Context<'_>, branch: Option<Node<'_>>) -> bool {
    branch
        .and_then(|b| statements(b).first().copied())
        .is_some_and(|v| c.text(v) == "nil")
}

fn next_value(c: &Context<'_>, node: Node<'_>, arg: &str) -> bool {
    let args = kids(node);
    node.kind() == "next" && args.len() == 1 && c.text(args[0]) == arg
}

mod guard;
use guard::trailing_guard;
pub(super) use guard::uncertain_guard;
