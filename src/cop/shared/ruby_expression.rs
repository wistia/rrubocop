//! Canonical expression keys for bounded AST parity checks and integer spelling normalization.
pub(crate) use super::integer_value as integer;
use super::{argument_nodes, call_method_name, call_receiver, named_kids, node_bytes, node_text};
use crate::parse::source::SourceFile;
use tree_sitter::Node;

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
        let arguments = argument_nodes(node);
        let keyword_start = arguments
            .iter()
            .rposition(|n| !matches!(n.kind(), "pair" | "hash_splat_argument"))
            .map_or(0, |i| i + 1);
        let mut args = arguments[..keyword_start]
            .iter()
            .map(|n| signature(source, *n))
            .collect::<Vec<_>>();
        if keyword_start < arguments.len() {
            let pairs = arguments[keyword_start..]
                .iter()
                .map(|n| signature(source, *n))
                .collect::<Vec<_>>()
                .join("|");
            args.push(format!("hash[{pairs}]"));
        }
        let block = node
            .child_by_field_name("block")
            .map(|n| signature(source, n));
        return format!("call:{receiver:?}:{method:?}:{safe}:{args:?}:{block:?}");
    }
    if let Some(value) = crate::cop::ported::static_literal(source, node) {
        return format!(
            "{}:{value:?}",
            if node.kind().contains("symbol")
                || node.kind() == "string"
                    && node.parent().is_some_and(
                        |p| p.kind() == "pair" && p.child_by_field_name("key") == Some(node)
                    )
                    && source.as_bytes().get(node.end_byte()) == Some(&b':')
            {
                "symbol"
            } else {
                "string"
            }
        );
    }
    if node.kind() == "regex" {
        return format!("regex:{}", node_text(source, node));
    }
    let kind = match node.kind() {
        "block" | "do_block" => "block",
        "block_body" | "body_statement" => "body",
        other => other,
    };
    if node.named_child_count() == 0
        && !matches!(
            kind,
            "block" | "body" | "hash" | "array" | "block_parameters"
        )
    {
        return format!("{kind}:{}", node_text(source, node));
    }
    let children = named_kids(node)
        .into_iter()
        .filter(|n| n.kind() != "comment")
        .map(|n| signature(source, n))
        .collect::<Vec<_>>()
        .join("|");
    if let Some(operator) = node.child_by_field_name("operator") {
        format!("{kind}:{}[{children}]", node_text(source, operator))
    } else {
        format!("{kind}[{children}]")
    }
}
