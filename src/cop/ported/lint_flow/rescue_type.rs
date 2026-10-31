use super::super::context::*;
use tree_sitter::Node;
pub(super) fn rescue_type(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let Some(ex) = field(n, "exceptions") else {
        return vec![];
    };
    let exceptions = kids(ex);
    let invalid = exceptions
        .iter()
        .filter(|e| invalid_exception(**e))
        .copied()
        .collect::<Vec<_>>();
    if invalid.is_empty() {
        return vec![];
    }
    vec![rescue_finding(c, n, ex, &exceptions, &invalid)]
}
fn invalid_exception(n: Node<'_>) -> bool {
    literal(n) && !matches!(n.kind(), "true" | "false" | "regex")
}
fn rescue_finding(
    c: &Context<'_>,
    n: Node<'_>,
    ex: Node<'_>,
    exceptions: &[Node<'_>],
    invalid: &[Node<'_>],
) -> Finding {
    let names = sources(c, invalid);
    Finding::range(n.start_byte(), ex.end_byte(), format!("Rescuing from `{names}` will raise a `TypeError` instead of catching the actual exception.")).edit(n.start_byte() + 6, ex.end_byte(), replacement(c, &valid_exceptions(exceptions,invalid)))
}
fn valid_exceptions<'a>(exceptions: &[Node<'a>], invalid: &[Node<'_>]) -> Vec<Node<'a>> {
    exceptions
        .iter()
        .filter(|e| !invalid.contains(e))
        .copied()
        .collect()
}

fn sources(c: &Context<'_>, nodes: &[Node<'_>]) -> String {
    nodes
        .iter()
        .map(|e| c.text(*e))
        .collect::<Vec<_>>()
        .join(", ")
}
fn replacement(c: &Context<'_>, valid: &[Node<'_>]) -> String {
    if valid.is_empty() {
        String::new()
    } else {
        format!(" {}", sources(c, valid))
    }
}
