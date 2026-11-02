use super::super::context::*;
use tree_sitter::Node;
pub(super) fn attr_comma(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    if !["attr", "attr_reader", "attr_writer", "attr_accessor"].contains(&c.method(n)) {
        return vec![];
    }
    let Some(offset) = trailing_comma(c, &semantic_args(n)) else {
        return vec![];
    };
    let finding = Finding::range(
        offset,
        offset + 1,
        "Avoid leaving a trailing comma in attribute declarations.",
    );
    vec![
        if field(n, "arguments").is_some_and(|a| c.text(a).starts_with('(')) {
            finding
        } else {
            finding.edit_with(c, offset, offset + 1, || "")
        },
    ]
}
fn trailing_comma(c: &Context<'_>, a: &[Node<'_>]) -> Option<usize> {
    if a.len() < 2 || a.last().is_none_or(|a| a.kind() != "method") {
        return None;
    }
    comma_between(c, a[a.len() - 2].end_byte(), a.last()?.start_byte())
}

fn comma_between(c: &Context<'_>, start: usize, end: usize) -> Option<usize> {
    c.source.as_bytes()[start..end]
        .iter()
        .position(|b| *b == b',')
        .map(|i| start + i)
}
