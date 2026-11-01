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
