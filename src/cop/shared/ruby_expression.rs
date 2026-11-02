//! Canonical expression keys for bounded AST parity checks and integer spelling normalization.
use super::{argument_nodes, call_method_name, call_receiver, node_bytes, node_text};
use crate::parse::source::SourceFile;
use num_bigint::BigInt;
use tree_sitter::Node;
pub(crate) fn integer(source: &SourceFile, node: Node<'_>) -> Option<BigInt> {
    if node.kind() != "integer"
        && !(node.kind() == "unary"
            && node
                .child_by_field_name("operator")
                .is_some_and(|n| matches!(node_text(source, n).as_str(), "+" | "-"))
            && node
                .child_by_field_name("operand")
                .is_some_and(|n| n.kind() == "integer"))
    {
        return None;
    }
    let value = node_text(source, node).replace('_', "");
    let negative = value.starts_with('-');
    let value = value.trim_start_matches(['+', '-']);
    let (radix, digits) = match value.get(..2) {
        Some("0x" | "0X") => (16, &value[2..]),
        Some("0b" | "0B") => (2, &value[2..]),
        Some("0o" | "0O") => (8, &value[2..]),
        Some("0d" | "0D") => (10, &value[2..]),
        _ if value.len() > 1 && value.starts_with('0') => (8, value),
        _ => (10, value),
    };
    let integer = BigInt::parse_bytes(digits.as_bytes(), radix)?;
    Some(if negative { -integer } else { integer })
}

pub(crate) fn signature(source: &SourceFile, node: Node<'_>) -> String {
    if let Some(value) = integer(source, node) {
        return format!("integer:{value}");
    }
    if node.kind() == "float" {
        if let Ok(value) = node_text(source, node).replace('_', "").parse::<f64>() {
            return format!("float:{value:?}");
        }
    }
    if node.kind() == "call"
        || node.kind() == "identifier" && !crate::cop::ported::local_identifier(source, node)
    {
        let receiver = call_receiver(node).map(|n| signature(source, n));
        let method = if node.kind() == "identifier" {
            node_text(source, node)
        } else {
            call_method_name(source, node)
                .map(|n| String::from_utf8_lossy(n).into_owned())
                .unwrap_or_else(|| "call".into())
        };
        let safe = node
            .child_by_field_name("operator")
            .is_some_and(|n| node_bytes(source, n) == b"&.");
        let args = argument_nodes(node)
            .into_iter()
            .map(|n| signature(source, n))
            .collect::<Vec<_>>();
        let block = node
            .child_by_field_name("block")
            .map(|n| signature(source, n));
        return format!("call:{receiver:?}:{method:?}:{safe}:{args:?}:{block:?}");
    }
    if let Some(value) = crate::cop::ported::static_literal(source, node) {
        return format!(
            "{}:{value:?}",
            if node.kind().contains("symbol") {
                "symbol"
            } else {
                "string"
            }
        );
    }
    if node.named_child_count() == 0 {
        return format!("{}:{}", node.kind(), node_text(source, node));
    }
    let mut cursor = node.walk();
    let children = node
        .children(&mut cursor)
        .filter(|n| n.kind() != "comment")
        .map(|n| signature(source, n))
        .collect::<Vec<_>>()
        .join("|");
    format!("{}[{children}]", node.kind())
}
