use super::*;
use num_bigint::BigInt;
mod rational;

pub(super) fn key(c: &Context<'_>, n: Node<'_>, equality: Equality) -> Option<String> {
    let (literal, negative) = operand(c, n)?;
    number(c, literal, negative, equality)
}
fn number(
    c: &Context<'_>,
    literal: Node<'_>,
    negative: bool,
    equality: Equality,
) -> Option<String> {
    let text = c.text(literal).replace('_', "");
    match literal.kind() {
        "integer" => {
            integer(&text).map(|value| format!("integer:{}", if negative { -value } else { value }))
        }
        "float" => float(&text, negative, equality),
        "rational" => rational::key(c, literal, negative),
        "complex" => number(c, literal.named_child(0)?, negative, equality)
            .map(|value| format!("complex:{value}")),
        _ => None,
    }
}
fn operand<'a>(c: &Context<'_>, n: Node<'a>) -> Option<(Node<'a>, bool)> {
    if matches!(n.kind(), "integer" | "float" | "rational" | "complex") {
        return Some((n, false));
    }
    if n.kind() != "unary" || !["+", "-"].contains(&operator(c, n)) {
        return None;
    }
    let literal = field(n, "operand")?;
    matches!(literal.kind(), "integer" | "float" | "rational" | "complex")
        .then_some((literal, operator(c, n) == "-"))
}
fn integer(text: &str) -> Option<BigInt> {
    let (radix, digits) = match text.get(..2) {
        Some("0x" | "0X") => (16, &text[2..]),
        Some("0b" | "0B") => (2, &text[2..]),
        Some("0o" | "0O") => (8, &text[2..]),
        Some("0d" | "0D") => (10, &text[2..]),
        _ if text.len() > 1 && text.starts_with('0') => (8, text),
        _ => (10, text),
    };
    BigInt::parse_bytes(digits.as_bytes(), radix)
}
fn float(text: &str, negative: bool, equality: Equality) -> Option<String> {
    let value = text.parse::<f64>().ok()?;
    let value = if negative { -value } else { value };
    Some(format!(
        "float:{}",
        if value == 0.0 && equality == Equality::Node {
            0
        } else {
            value.to_bits()
        }
    ))
}
