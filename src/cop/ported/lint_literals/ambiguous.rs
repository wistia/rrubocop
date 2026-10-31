use super::super::context::*;
use tree_sitter::Node;
pub(super) fn ambiguous(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let Some((offset, op)) = assignment_mistake(c, n) else {
        return vec![];
    };
    vec![Finding::range(
        offset,
        offset + 2,
        format!("Suspicious assignment detected. Did you mean `{op}`?"),
    )]
}
fn assignment_mistake(c: &Context<'_>, n: Node<'_>) -> Option<(usize, &'static str)> {
    let rhs = field(n, "right")?;
    let lhs = field(n, "left")?;
    let (eq, op) = mistake_in(&c.source.as_bytes()[lhs.end_byte()..rhs.end_byte()])?;
    Some((lhs.end_byte() + eq, op))
}
fn correction(pair: &[u8]) -> Option<&'static str> {
    match pair {
        b"=-" => Some("-="),
        b"=+" => Some("+="),
        b"=*" => Some("*="),
        b"=!" => Some("!="),
        _ => None,
    }
}

fn mistake_in(bytes: &[u8]) -> Option<(usize, &'static str)> {
    let eq = bytes.iter().position(|b| *b == b'=')?;
    correction(bytes.get(eq..eq + 2)?).map(|op| (eq, op))
}
