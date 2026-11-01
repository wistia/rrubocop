use super::*;

pub(super) fn trailing_guard(yes: Option<Node<'_>>, no: Option<Node<'_>>) -> Option<bool> {
    if no.is_none() {
        return single_next(yes).map(|node| !kids(node).is_empty());
    }
    if yes.is_none() {
        return single_next(no).map(|node| kids(node).is_empty());
    }
    None
}

fn single_next(branch: Option<Node<'_>>) -> Option<Node<'_>> {
    let branch = branch?;
    let body = statements(branch);
    (body.len() == 1 && body[0].kind() == "next").then(|| body[0])
}

pub(in crate::cop::ported::style_blocks::map_compact) fn uncertain_guard(
    c: &Context<'_>,
    map: Node<'_>,
) -> bool {
    let Some(statements) = block(map).and_then(body).map(statements) else {
        return false;
    };
    if statements.len() != 2 || c.text(statements[1]) == "nil" {
        return false;
    }
    let Some((yes, no)) = map_branches(statements[0]) else {
        return false;
    };
    [yes, no]
        .into_iter()
        .filter_map(single_next)
        .any(|node| !kids(node).is_empty())
}
