use super::super::context::*;
use super::arguments as args;
use num_bigint::BigInt;
use tree_sitter::Node;
mod integer;
pub(super) fn check(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    times_count(c, n)
        .map(|count| Finding::new(n, format!("Useless call to `{count}.times` detected.")))
        .into_iter()
        .collect()
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
