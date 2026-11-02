use super::super::context::*;
use num_bigint::BigInt;
use tree_sitter::Node;
pub(super) fn value(c: &Context<'_>, n: Node<'_>) -> Option<BigInt> {
    if !integer_literal(n) {
        return None;
    }
    let text = c.text(n).replace('_', "");
    let (radix, digits) = digits(text.strip_prefix(['-', '+']).unwrap_or(&text));
    let number = BigInt::parse_bytes(digits.as_bytes(), radix)?;
    Some(if text.starts_with('-') {
        -number
    } else {
        number
    })
}
fn integer_literal(n: Node<'_>) -> bool {
    n.kind() == "integer"
        || n.kind() == "unary"
            && field(n, "operand").is_some_and(|operand| operand.kind() == "integer")
}
fn digits(text: &str) -> (u32, &str) {
    match text.get(..2).unwrap_or("") {
        "0x" | "0X" => (16, &text[2..]),
        "0o" | "0O" => (8, &text[2..]),
        "0b" | "0B" => (2, &text[2..]),
        "0d" | "0D" => (10, &text[2..]),
        _ if text.len() > 1 && text.starts_with('0') => (8, text),
        _ => (10, text),
    }
}
