use super::super::context::*;
use tree_sitter::Node;

pub(super) fn safe_empty(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let Some(condition) =
        field(n, "condition").filter(|condition| unsafe_empty_check(c, *condition))
    else {
        return vec![];
    };
    let receiver = c.receiver(condition);
    vec![
        Finding::new(
            condition,
            "Avoid calling `empty?` with the safe navigation operator in conditionals.",
        )
        .replace(condition, format!("{receiver} && {receiver}.empty?")),
    ]
}

fn unsafe_empty_check(c: &Context<'_>, n: Node<'_>) -> bool {
    safe(c, n)
        && c.method(n) == "empty?"
        && field(n, "receiver").is_some_and(|receiver| unsafe_receiver(c, receiver))
}

fn unsafe_receiver(c: &Context<'_>, n: Node<'_>) -> bool {
    (is_call(n) || n.kind() == "identifier")
        && !safe(c, n)
        && !(n.kind() == "identifier" && local_identifier(c, n))
}
