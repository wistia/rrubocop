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
    associated_sign(source, outer).unwrap_or(node.start_byte())
}

fn power_left(source: &SourceFile, parent: Node<'_>, child: Node<'_>) -> bool {
    parent.kind() == "binary"
        && parent.child_by_field_name("left") == Some(child)
        && parent
            .child_by_field_name("operator")
            .is_some_and(|operator| node_bytes(source, operator) == b"**")
}

fn associated_sign(source: &SourceFile, mut outer: Node<'_>) -> Option<usize> {
    let receiver_chain = matches!(
        outer.kind(),
        "call" | "element_reference" | "scope_resolution"
    );
    if receiver_chain {
        while let Some(parent) = outer
            .parent()
            .filter(|parent| power_left(source, *parent, outer))
        {
            outer = parent;
        }
    }
    let parent = outer.parent()?;
    if parent.kind() == "unary"
        && parent.child_by_field_name("operand") == Some(outer)
        && parent
            .child_by_field_name("operator")
            .is_some_and(|operator| matches!(node_bytes(source, operator), b"+" | b"-"))
    {
        if !receiver_chain
            && parent
                .parent()
                .is_some_and(|power| power_left(source, power, parent))
        {
            return None;
        }
        return Some(parent.start_byte());
    }
    command_unary_plus(source, parent)
        .filter(|command| command.right == outer)
        .map(|command| command.operator.start_byte())
}

pub fn float_literal_span(source: &SourceFile, node: Node<'_>) -> Option<(usize, usize)> {
    if node.kind() != "float"
        || node
            .parent()
            .is_some_and(|parent| matches!(parent.kind(), "rational" | "complex"))
    {
        return None;
    }
    let mut outer = node;
    while let Some(parent) = outer.parent().filter(|parent| {
        parent.kind() == "call" && call_receiver(*parent) == Some(outer)
            || parent.kind() == "element_reference"
                && parent.child_by_field_name("object") == Some(outer)
            || parent.kind() == "scope_resolution"
                && parent.child_by_field_name("scope") == Some(outer)
    }) {
        outer = parent;
    }
    Some((
        associated_sign(source, outer).unwrap_or(node.start_byte()),
        node.end_byte(),
    ))
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
    #[test]
    fn float_literal_roles_and_ranges_match_original_parser() {
        let rows: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/core-float-literal-spans.json"
        )))
        .unwrap();
        let mut failures = Vec::new();
        for row in rows.as_array().unwrap() {
            let source = SourceFile::from_bytes(
                "probe.rb",
                row["source"].as_str().unwrap().as_bytes().to_vec(),
            );
            let tree = crate::parse::parse_ruby(&source).unwrap();
            assert!(
                !tree.root_node().has_error(),
                "{} {}",
                row["source"],
                tree.root_node().to_sexp()
            );
            let mut stack = vec![tree.root_node()];
            let mut actual = Vec::new();
            let mut calls = Vec::new();
            while let Some(node) = stack.pop() {
                if let Some((start, end)) = float_literal_span(&source, node) {
                    let (line, column) = source.offset_to_line_col(start);
                    let text = String::from_utf8_lossy(&source.as_bytes()[start..end])
                        .replace("\r\n", "\n");
                    actual.push(serde_json::json!({"line":line,"column":column+1,"length":text.chars().count(),"source":text}));
                }
                if node.kind() == "call" && node.child_by_field_name("operator").is_some() {
                    let selector = node.child_by_field_name("method").unwrap();
                    let (line, column) = source.offset_to_line_col(selector.start_byte());
                    let (start_line, start_column) =
                        source.offset_to_line_col(numeric_send_start(&source, node));
                    calls.push(serde_json::json!({"selector":[line,column],"start":[start_line,start_column]}));
                }
                stack.extend(super::super::named_kids(node));
            }
            calls.sort_by_key(|row| (row["selector"][0].as_u64(), row["selector"][1].as_u64()));
            let mut expected_calls = row["calls"].as_array().unwrap().clone();
            expected_calls
                .sort_by_key(|row| (row["selector"][0].as_u64(), row["selector"][1].as_u64()));
            if calls != expected_calls {
                failures.push(serde_json::json!({"source":row["source"],"calls":calls,"expected_calls":expected_calls}));
            }
            actual.sort_by_key(|row| (row["line"].as_u64(), row["column"].as_u64()));
            let mut expected:Vec<_>=row["floats"].as_array().unwrap().iter().map(|literal|serde_json::json!({"line":literal["line"],"column":literal["column"],"length":literal["length"],"source":literal["source"]})).collect();
            expected.sort_by_key(|row| (row["line"].as_u64(), row["column"].as_u64()));
            if actual != expected {
                failures.push(
                    serde_json::json!({"source":row["source"],"actual":actual,"expected":expected}),
                );
            }
        }
        assert!(
            failures.is_empty(),
            "{}",
            serde_json::to_string_pretty(&failures).unwrap()
        );
    }
}
