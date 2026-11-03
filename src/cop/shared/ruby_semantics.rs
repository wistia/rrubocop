use super::{integer_value, named_kids, node_bytes};
use crate::parse::source::SourceFile;
use tree_sitter::Node;

fn children(node: Node<'_>) -> Vec<Node<'_>> {
    named_kids(node)
        .into_iter()
        .filter(|node| !matches!(node.kind(), "comment" | "heredoc_body" | "empty_statement"))
        .collect()
}
fn heredoc_pure(source: &SourceFile, mut node: Node<'_>) -> bool {
    if node_bytes(source, node).contains(&b'`') {
        return false;
    }
    let Some((start, end)) = crate::cop::ported::heredoc_span(source, node) else {
        return false;
    };
    while let Some(parent) = node.parent() {
        node = parent;
    }
    let mut pending = vec![node];
    while let Some(node) = pending.pop() {
        if node.kind() == "interpolation" && node.start_byte() >= start && node.end_byte() <= end {
            if !pure(source, node) {
                return false;
            }
        } else {
            pending.extend(named_kids(node));
        }
    }
    true
}
pub fn pure(source: &SourceFile, node: Node<'_>) -> bool {
    if integer_value(source, node).is_some()
        || super::ruby_expression::float_value(source, node).is_some()
    {
        return true;
    }
    match node.kind() {
        "constant" | "scope_resolution" | "instance_variable" | "class_variable"
        | "global_variable" | "true" | "false" | "nil" | "simple_symbol" | "hash_key_symbol"
        | "regex_options" => true,
        "identifier" => {
            matches!(
                node_bytes(source, node),
                b"__FILE__" | b"__LINE__" | b"__ENCODING__"
            ) || crate::cop::ported::local_identifier(source, node)
        }
        "heredoc_beginning" => heredoc_pure(source, node),
        "string" | "delimited_symbol" | "regex" | "bare_string" | "bare_symbol" => {
            children(node).into_iter().all(|child| {
                matches!(
                    child.kind(),
                    "string_content" | "escape_sequence" | "regex_options"
                ) || pure(source, child)
            })
        }
        "unary" => node
            .child_by_field_name("operator")
            .is_some_and(|operator| node_bytes(source, operator) == b"defined?"),
        "binary"
            if node.child_by_field_name("operator").is_some_and(|op| {
                matches!(node_bytes(source, op), b"&&" | b"and" | b"||" | b"or")
            }) =>
        {
            children(node).into_iter().all(|child| pure(source, child))
        }
        "pattern" => children(node).into_iter().all(|child| pure(source, child)),
        "pair" if node.child_by_field_name("value").is_none() => {
            node.child_by_field_name("key").is_some_and(|key| {
                matches!(
                    super::ruby_expression::shorthand_value(source, key),
                    Some(
                        super::ruby_expression::ShorthandValue::Constant
                            | super::ruby_expression::ShorthandValue::Local
                    )
                )
            })
        }
        "array"
        | "hash"
        | "pair"
        | "range"
        | "parenthesized_statements"
        | "interpolation"
        | "conditional"
        | "if"
        | "case"
        | "when"
        | "elsif"
        | "if_modifier"
        | "unless_modifier"
        | "while_modifier"
        | "until_modifier"
        | "unless"
        | "then"
        | "else"
        | "while"
        | "until"
        | "do"
        | "body_statement"
        | "block_body"
        | "begin"
        | "string_array"
        | "symbol_array"
        | "ensure" => children(node).into_iter().all(|child| pure(source, child)),
        _ => false,
    }
}

pub fn value_used(source: &SourceFile, node: Node<'_>) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    match parent.kind() {
        "program" => false,
        "body_statement"
        | "block_body"
        | "then"
        | "else"
        | "do"
        | "parenthesized_statements"
        | "begin" => {
            let values = children(parent);
            if values
                .iter()
                .any(|child| matches!(child.kind(), "rescue" | "ensure"))
            {
                if node.kind() == "else" {
                    return true;
                }
                return values
                    .iter()
                    .take_while(|child| !matches!(child.kind(), "rescue" | "ensure" | "else"))
                    .last()
                    == Some(&node);
            }
            if values.last() != Some(&node) {
                return false;
            }
            value_used(source, parent)
        }
        "array" | "hash" | "pair" | "range" | "string" | "delimited_symbol" | "interpolation"
        | "regex" | "subshell" | "when" => value_used(source, parent),
        "pattern" if parent.parent().is_some_and(|node| node.kind() == "when") => {
            value_used(source, parent)
        }
        "unary"
            if parent
                .child_by_field_name("operator")
                .is_some_and(|op| node_bytes(source, op) == b"defined?") =>
        {
            value_used(source, parent)
        }
        "if" | "unless" | "elsif" | "conditional" | "if_modifier" | "unless_modifier" => {
            parent.child_by_field_name("condition") == Some(node) || value_used(source, parent)
        }
        "case" => parent.child_by_field_name("value") == Some(node) || value_used(source, parent),
        "while" | "until" | "while_modifier" | "until_modifier" => {
            parent.child_by_field_name("condition") == Some(node)
        }
        "for" => parent.child_by_field_name("body") != Some(node) || value_used(source, parent),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::shared::call_method_name;
    #[derive(serde::Deserialize)]
    struct Observation {
        source: String,
        pure: Option<bool>,
        value_used: Option<bool>,
    }
    #[test]
    fn expression_properties_match_pinned_original_ruby() {
        let observations: Vec<Observation> = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/performance-seven-upstream/expression-properties.json"
        )))
        .unwrap();
        for observation in observations {
            let source =
                SourceFile::from_bytes("properties.rb", observation.source.as_bytes().to_vec());
            let tree = crate::parse::parse_ruby(&source).unwrap();
            let mut pending = vec![tree.root_node()];
            let mut target = None;
            while let Some(node) = pending.pop() {
                if call_method_name(&source, node) == Some(b"probe") {
                    target = Some(super::super::argument_nodes(node)[0]);
                    break;
                }
                if call_method_name(&source, node) == Some(b"target") {
                    target = Some(node);
                    break;
                }
                pending.extend(named_kids(node).into_iter().rev());
            }
            let target = target.expect(&observation.source);
            if let Some(expected) = observation.pure {
                assert_eq!(pure(&source, target), expected, "{}", observation.source);
            }
            if let Some(expected) = observation.value_used {
                assert_eq!(
                    value_used(&source, target),
                    expected,
                    "{}",
                    observation.source
                );
            }
        }
    }
}
