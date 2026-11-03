//! Source geometry for numeric signs folded into receivers by the original Ruby parser.
use super::{call_receiver, command_unary_plus, node_bytes};
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub fn numeric_send_start(source: &SourceFile, node: Node<'_>) -> usize {
    if node.kind() != "call" {
        return node.start_byte();
    }
    let mut base = node;
    while base.kind() == "call" {
        let Some(receiver) = call_receiver(base) else {
            return node.start_byte();
        };
        base = receiver;
    }
    if !matches!(base.kind(), "integer" | "float" | "rational" | "complex") {
        return node.start_byte();
    }
    let mut outer = node;
    while let Some(parent) = outer
        .parent()
        .filter(|parent| parent.kind() == "call" && call_receiver(*parent) == Some(outer))
    {
        outer = parent;
    }
    let Some(parent) = outer.parent() else {
        return node.start_byte();
    };
    if parent.kind() == "unary"
        && parent.child_by_field_name("operand") == Some(outer)
        && parent
            .child_by_field_name("operator")
            .is_some_and(|operator| matches!(node_bytes(source, operator), b"+" | b"-"))
    {
        return parent.start_byte();
    }
    if let Some(command) =
        command_unary_plus(source, parent).filter(|command| command.right == outer)
    {
        return command.operator.start_byte();
    }
    node.start_byte()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn folded_numeric_send_spans_match_original_parser() {
        let rows: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/framework-numeric-send-spans.json"
        )))
        .unwrap();
        for row in rows.as_array().unwrap() {
            let source = SourceFile::from_bytes(
                "number.rb",
                row["source"].as_str().unwrap().as_bytes().to_vec(),
            );
            let tree = crate::parse::parse_ruby(&source).unwrap();
            assert!(!tree.root_node().has_error(), "{}", row["source"]);
            let mut stack = vec![tree.root_node()];
            let mut got = Vec::new();
            while let Some(node) = stack.pop() {
                if node.kind() == "call" && node.child_by_field_name("operator").is_some() {
                    let selector = node.child_by_field_name("method").unwrap();
                    let start = numeric_send_start(&source, node);
                    let (line, column) = source.offset_to_line_col(selector.start_byte());
                    let (start_line, start_column) = source.offset_to_line_col(start);
                    got.push(serde_json::json!({"selector":[line,column],"start":[start_line,start_column]}));
                }
                stack.extend(super::super::named_kids(node));
            }
            got.sort_by_key(|row| (row["selector"][0].as_u64(), row["selector"][1].as_u64()));
            let expected: Vec<_> = row["calls"]
                .as_array()
                .unwrap()
                .iter()
                .map(|call| serde_json::json!({"selector":call["selector"],"start":call["start"]}))
                .collect();
            assert_eq!(
                serde_json::json!(got),
                serde_json::json!(expected),
                "{}",
                row["source"]
            );
        }
    }
}
