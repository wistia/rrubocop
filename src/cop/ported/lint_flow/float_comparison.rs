use super::super::context::*;
use tree_sitter::Node;
pub(super) fn float_comparison(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let Some((left, right)) = comparison(c, n) else {
        return vec![];
    };
    if literal_zero(c, left) || literal_zero(c, right) {
        return vec![];
    }
    if floating(c, left) || floating(c, right) {
        vec![Finding::new(
            n,
            "Avoid (in)equality comparisons of floats as they are unreliable.",
        )]
    } else {
        vec![]
    }
}
fn comparison<'a>(c: &Context<'_>, n: Node<'a>) -> Option<(Node<'a>, Node<'a>)> {
    if n.kind() == "binary" && ["==", "!="].contains(&operator(c, n)) {
        field(n, "left").zip(field(n, "right"))
    } else if ["==", "!=", "eql?", "equal?"].contains(&c.method(n)) {
        field(n, "receiver").zip(semantic_args(n).first().copied())
    } else {
        None
    }
}
fn literal_zero(c: &Context<'_>, n: Node<'_>) -> bool {
    matches!(n.kind(), "integer" | "float") && c.text(n).parse::<f64>() == Ok(0.0)
}
fn floating(c: &Context<'_>, n: Node<'_>) -> bool {
    match n.kind() {
        "float" => true,
        "parenthesized_statements" | "unary" => kids(n).first().is_some_and(|a| floating(c, *a)),
        "binary" if ["+", "-", "*", "/", "%", "**"].contains(&operator(c, n)) => {
            kids(n).iter().any(|a| floating(c, *a))
        }
        "call" => float_call(c, n),
        _ => false,
    }
}
fn float_call(c: &Context<'_>, n: Node<'_>) -> bool {
    if ["to_f", "Float", "fdiv"].contains(&c.method(n)) {
        return true;
    }
    let Some(recv) = field(n, "receiver").filter(|r| r.kind() == "float") else {
        return false;
    };
    match c.method(n) {
        "abs" | "magnitude" | "modulo" | "next_float" | "prev_float" | "quo" => true,
        "angle" | "arg" | "phase" => c.text(recv).parse::<f64>().is_ok_and(|f| f < 0.0),
        "ceil" | "floor" | "round" | "truncate" => positive_precision(c, n),
        _ => false,
    }
}
fn positive_precision(c: &Context<'_>, n: Node<'_>) -> bool {
    semantic_args(n)
        .first()
        .is_some_and(|a| a.kind() == "integer" && c.text(*a).parse::<i64>().is_ok_and(|v| v > 0))
}
