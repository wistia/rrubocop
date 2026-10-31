use super::super::context::*;
use tree_sitter::Node;
pub(super) fn check(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let parameters = kids(n);
    let required = keywords(&parameters, false);
    let optional = keywords(&parameters, true);
    let Some(first) = optional.first() else {
        return vec![];
    };
    optional
        .iter()
        .filter(|p| required_after(&required, **p))
        .map(|p| {
            let f = Finding::new(
                *p,
                "Place optional keyword parameters at the end of the parameters list.",
            );
            if p != first {
                return f;
            }
            move_required(c, n, &parameters, &required, &optional, f)
        })
        .collect()
}
fn keywords<'n>(parameters: &[Node<'n>], optional: bool) -> Vec<Node<'n>> {
    parameters
        .iter()
        .filter(|p| p.kind() == "keyword_parameter" && field(**p, "value").is_some() == optional)
        .copied()
        .collect()
}
fn move_required(
    c: &Context<'_>,
    n: Node<'_>,
    parameters: &[Node<'_>],
    required: &[Node<'_>],
    optional: &[Node<'_>],
    f: Finding,
) -> Finding {
    let mut f = insert_required(c, optional[0], required, f);
    if newline_needed(c, n, parameters) {
        let last = optional.last().unwrap();
        f = f.also_edit(last.end_byte(), last.end_byte(), "\n");
    }
    f
}
fn newline_needed(c: &Context<'_>, n: Node<'_>, parameters: &[Node<'_>]) -> bool {
    !c.text(n).starts_with(['(', '|'])
        && n.kind() == "method_parameters"
        && !line_break_after(c, n)
        && parameters
            .last()
            .is_some_and(|p| p.kind() == "keyword_parameter")
}
fn removal(c: &Context<'_>, n: Node<'_>) -> (usize, usize) {
    let bytes = c.source.as_bytes();
    let mut start = n.start_byte();
    let mut end = n.end_byte();
    while start > 0 && bytes[start - 1].is_ascii_whitespace() {
        start -= 1;
    }
    if start > 0 && bytes[start - 1] == b',' {
        start -= 1;
    }
    while end < bytes.len() && matches!(bytes[end], b' ' | b'\t') {
        end += 1;
    }
    (start, end)
}

fn required_after(required: &[Node<'_>], n: Node<'_>) -> bool {
    required.iter().any(|r| r.start_byte() > n.start_byte())
}
fn insert_required(c: &Context<'_>, first: Node<'_>, required: &[Node<'_>], f: Finding) -> Finding {
    let moved = required
        .iter()
        .filter(|r| r.start_byte() > first.start_byte())
        .copied()
        .collect::<Vec<_>>();
    let names = keyword_sources(c, &moved);
    moved.into_iter().fold(
        f.edit(first.start_byte(), first.start_byte(), format!("{names}, ")),
        |f, r| {
            let (start, end) = removal(c, r);
            f.also_edit(start, end, "")
        },
    )
}
fn keyword_sources(c: &Context<'_>, nodes: &[Node<'_>]) -> String {
    nodes
        .iter()
        .map(|n| c.text(*n))
        .collect::<Vec<_>>()
        .join(", ")
}

fn line_break_after(c: &Context<'_>, n: Node<'_>) -> bool {
    c.source.as_bytes()[n.end_byte()..]
        .iter()
        .find(|b| !matches!(b, b' ' | b'\t'))
        .is_some_and(|b| matches!(b, b'\n' | b'\r'))
}
