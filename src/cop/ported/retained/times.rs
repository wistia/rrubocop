use super::super::context::*;
use super::arguments as args;
use num_bigint::BigInt;
use tree_sitter::Node;
mod correction;
mod integer;
mod safety;
pub(super) fn check(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let Some(count) = times_count(c, n) else {
        return vec![];
    };
    let f = Finding::new(n, format!("Useless call to `{count}.times` detected."));
    if !own_line(c, n) || !safety::statement(n) || !safety::call(c, n) {
        return vec![f];
    }
    vec![correct(c, n, count, f)]
}
fn times_count(c: &Context<'_>, n: Node<'_>) -> Option<BigInt> {
    if c.method(n) != "times" || !valid_arguments(c, n) {
        return None;
    }
    let receiver = field(n, "receiver").filter(|r| r.kind() == "integer" || r.kind() == "unary")?;
    integer::parse(c.text(receiver)).filter(|i| *i <= BigInt::from(1))
}
fn valid_arguments(c: &Context<'_>, n: Node<'_>) -> bool {
    let a = args(n);
    a.len() <= 1
        && a.first().is_none_or(|a| {
            a.kind() == "block_argument"
                && kids(*a)
                    .first()
                    .is_some_and(|s| c.static_symbol(*s).is_some())
        })
}
fn correct(c: &Context<'_>, n: Node<'_>, count: BigInt, f: Finding) -> Finding {
    if !args(n).is_empty() {
        return f;
    }
    if never_process(count, n) {
        return if safety::removal(c, n) {
            remove_line(c, n, f)
        } else {
            f
        };
    }
    let Some((b, body)) = block_body(n) else {
        return f;
    };
    correction::correct_body(c, n, b, body, f)
}
fn own_line(c: &Context<'_>, n: Node<'_>) -> bool {
    let start = n.start_byte() - n.start_position().column;
    c.source.as_bytes()[start..n.start_byte()]
        .iter()
        .all(u8::is_ascii_whitespace)
}
fn remove_line(c: &Context<'_>, n: Node<'_>, f: Finding) -> Finding {
    let bytes = c.source.as_bytes();
    let start = n.start_byte() - n.start_position().column;
    f.edit_with(
        c,
        start,
        bytes[n.end_byte()..]
            .iter()
            .position(|b| *b == b'\n')
            .map(|p| n.end_byte() + p + 1)
            .unwrap_or(bytes.len()),
        || "",
    )
}

fn block_body(n: Node<'_>) -> Option<(Node<'_>, Node<'_>)> {
    let b = block(n)?;
    Some((b, field(b, "body")?))
}

fn never_process(count: BigInt, n: Node<'_>) -> bool {
    count < BigInt::from(1) || block(n).is_some_and(|b| field(b, "body").is_none())
}
