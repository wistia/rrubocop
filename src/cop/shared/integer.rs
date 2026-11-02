//! Ruby integer literal values without executing source.

use num_bigint::BigInt;
use tree_sitter::Node;

use super::node_bytes;
use crate::parse::source::SourceFile;

pub fn integer_value(source: &SourceFile, node: Node<'_>) -> Option<BigInt> {
    if node.kind() != "integer"
        && !(node.kind() == "unary"
            && node
                .child_by_field_name("operand")
                .is_some_and(|operand| operand.kind() == "integer"))
    {
        return None;
    }
    let text = std::str::from_utf8(node_bytes(source, node))
        .ok()?
        .replace('_', "");
    let unsigned = text.strip_prefix(['-', '+']).unwrap_or(&text);
    let (radix, digits) = match unsigned.get(..2).unwrap_or("") {
        "0x" | "0X" => (16, &unsigned[2..]),
        "0o" | "0O" => (8, &unsigned[2..]),
        "0b" | "0B" => (2, &unsigned[2..]),
        "0d" | "0D" => (10, &unsigned[2..]),
        _ if unsigned.len() > 1 && unsigned.starts_with('0') => (8, unsigned),
        _ => (10, unsigned),
    };
    let value = BigInt::parse_bytes(digits.as_bytes(), radix)?;
    Some(if text.starts_with('-') { -value } else { value })
}
