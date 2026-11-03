use std::collections::HashSet;
use tree_sitter::{Node, Tree};

use crate::cop::shared::{call_method_name, call_receiver, named_kids, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::{codemap::CodeMap, source::SourceFile};

pub struct LiteralAsCondition;

fn children(node: Node<'_>) -> Vec<Node<'_>> {
    named_kids(node)
        .into_iter()
        .filter(|child| child.kind() != "comment")
        .collect()
}
fn operator<'a>(source: &'a SourceFile, node: Node<'_>) -> &'a [u8] {
    node.child_by_field_name("operator")
        .map(|op| node_bytes(source, op))
        .unwrap_or_default()
}
fn numeric(node: Node<'_>) -> bool {
    matches!(node.kind(), "integer" | "float" | "rational" | "complex")
}
fn literal(source: &SourceFile, node: Node<'_>, condition: bool) -> bool {
    match node.kind() {
        "file" | "line" | "character" | "integer" | "float" | "rational" | "complex" | "string"
        | "chained_string" | "simple_symbol" | "delimited_symbol" | "array" | "hash"
        | "string_array" | "symbol_array" | "true" | "false" | "nil" | "subshell"
        | "heredoc_beginning" => true,
        "identifier" => matches!(node_bytes(source, node), b"__FILE__" | b"__LINE__"),
        "regex" | "range" => !condition,
        "unary" => {
            matches!(operator(source, node), b"+" | b"-")
                && node.child_by_field_name("operand").is_some_and(numeric)
        }
        _ => false,
    }
}
fn primitive_array(source: &SourceFile, node: Node<'_>) -> bool {
    children(node).into_iter().all(|child| {
        if matches!(child.kind(), "array" | "string_array" | "symbol_array") {
            return primitive_array(source, child);
        }
        if matches!(
            child.kind(),
            "string" | "delimited_symbol" | "heredoc_beginning"
        ) {
            return crate::cop::ported::ruby_string_parts(source, child)
                .is_some_and(|parts| parts.len() == 1);
        }
        if matches!(child.kind(), "bare_string" | "bare_symbol") {
            let mut stack = vec![child];
            while let Some(node) = stack.pop() {
                if node.kind() == "interpolation" {
                    return false;
                }
                stack.extend(children(node));
            }
            return true;
        }
        literal(source, child, false)
            && !matches!(
                child.kind(),
                "hash" | "range" | "regex" | "chained_string" | "subshell" | "heredoc_beginning"
            )
    })
}
fn handle(
    source: &SourceFile,
    node: Node<'_>,
    condition: bool,
    ranges: &mut HashSet<(usize, usize)>,
) {
    if literal(source, node, condition) {
        ranges.insert((node.start_byte(), node.end_byte()));
    } else {
        check(source, node, condition, ranges);
    }
}
fn check(
    source: &SourceFile,
    node: Node<'_>,
    condition: bool,
    ranges: &mut HashSet<(usize, usize)>,
) {
    match node.kind() {
        "unary" if operator(source, node) == b"!" => {
            if let Some(operand) = node.child_by_field_name("operand") {
                handle(source, operand, true, ranges);
            }
        }
        "binary" if matches!(operator(source, node), b"&&" | b"||" | b"and" | b"or") => {
            for field in ["left", "right"] {
                if let Some(child) = node.child_by_field_name(field) {
                    handle(source, child, condition, ranges);
                }
            }
        }
        "parenthesized_statements" => {
            let body = children(node);
            if body.len() == 1 {
                handle(source, body[0], condition, ranges);
            }
        }
        _ => {}
    }
}
fn pattern_binding(node: Node<'_>) -> bool {
    match node.kind() {
        "identifier" => true,
        "variable_reference_pattern"
        | "expression_reference_pattern"
        | "string"
        | "delimited_symbol"
        | "regex" => false,
        "keyword_pattern" => node
            .child_by_field_name("value")
            .is_none_or(pattern_binding),
        _ => children(node).into_iter().any(pattern_binding),
    }
}
fn case_has_binding(node: Node<'_>) -> bool {
    if node.kind() == "in_clause"
        && node
            .child_by_field_name("pattern")
            .is_some_and(pattern_binding)
    {
        return true;
    }
    children(node).into_iter().any(case_has_binding)
}
impl Cop for LiteralAsCondition {
    fn name(&self) -> &'static str {
        "Lint/LiteralAsCondition"
    }
    fn default_severity(&self) -> Severity {
        Severity::Warning
    }
    fn uses_source_phase(&self) -> bool {
        true
    }
    fn check_source(
        &self,
        source: &SourceFile,
        tree: &Tree,
        _map: &CodeMap,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let mut ranges = HashSet::new();
        let mut stack = vec![tree.root_node()];
        while let Some(node) = stack.pop() {
            stack.extend(children(node).into_iter().rev());
            match node.kind() {
                "if" | "elsif" | "unless" | "if_modifier" | "unless_modifier" | "conditional"
                | "while" | "until" | "while_modifier" | "until_modifier" => {
                    let Some(cond) = node.child_by_field_name("condition") else {
                        continue;
                    };
                    if (matches!(node.kind(), "while" | "while_modifier") && cond.kind() == "true")
                        || (matches!(node.kind(), "until" | "until_modifier")
                            && cond.kind() == "false")
                    {
                        continue;
                    }
                    handle(source, cond, true, &mut ranges);
                }
                "unary" if matches!(operator(source, node), b"!" | b"not") => {
                    if let Some(operand) = node.child_by_field_name("operand") {
                        handle(source, operand, true, &mut ranges);
                    }
                }
                "call"
                    if call_method_name(source, node) == Some(b"!")
                        && operator(source, node) != b"&." =>
                {
                    if let Some(receiver) = call_receiver(node) {
                        handle(source, receiver, false, &mut ranges);
                    }
                }
                "case" | "case_match" => {
                    if let Some(value) = node.child_by_field_name("value") {
                        if node.kind() == "case_match" && case_has_binding(node) {
                            continue;
                        }
                        if matches!(value.kind(), "array" | "string_array" | "symbol_array")
                            && !primitive_array(source, value)
                        {
                            continue;
                        }
                        if matches!(
                            value.kind(),
                            "string" | "chained_string" | "heredoc_beginning"
                        ) && crate::cop::ported::ruby_string_parts(source, value)
                            .is_none_or(|parts| parts.len() != 1)
                        {
                            continue;
                        }
                        handle(source, value, false, &mut ranges);
                    } else if node.kind() == "case" {
                        for branch in children(node)
                            .into_iter()
                            .filter(|child| child.kind() == "when")
                        {
                            let patterns: Vec<_> = children(branch)
                                .into_iter()
                                .filter(|child| child.kind() == "pattern")
                                .flat_map(children)
                                .collect();
                            if !patterns.is_empty()
                                && patterns.iter().all(|child| literal(source, *child, false))
                            {
                                ranges.insert((
                                    patterns[0].start_byte(),
                                    patterns.last().unwrap().end_byte(),
                                ));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        let mut ranges: Vec<_> = ranges.into_iter().collect();
        ranges.sort_unstable();
        for (start, end) in ranges {
            let (line, column) = source.offset_to_line_col(start);
            let text =
                String::from_utf8_lossy(&source.as_bytes()[start..end]).replace("\r\n", "\n");
            let mut diagnostic = self.diagnostic(
                source,
                line,
                column,
                format!("Literal `{text}` appeared as a condition."),
            );
            diagnostic.highlight_length = text.chars().count();
            diagnostics.push(diagnostic);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(LiteralAsCondition, "cops/lint/literal_as_condition");
    #[test]
    fn independent_original_states() {
        let fixture = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/lint/literal_as_condition/original-independent.json"
        ));
        let cases: serde_json::Value = serde_json::from_str(fixture).unwrap();
        let mut failures = Vec::new();
        for case in cases.as_array().unwrap() {
            let source = SourceFile::from_bytes(
                "probe.rb",
                case["original"].as_str().unwrap().as_bytes().to_vec(),
            );
            let tree = crate::parse::parse_ruby(&source).unwrap();
            if tree.root_node().has_error() {
                failures.push(serde_json::json!({"id":case["id"],"source":case["original"],"tree":tree.root_node().to_sexp()}));
            }
        }
        assert!(
            failures.is_empty(),
            "NATIVE_ERROR_COVERAGE {}",
            serde_json::to_string(&failures).unwrap()
        );
        crate::testutil::assert_original_spec_parity(&LiteralAsCondition, fixture);
    }
    #[test]
    fn unchanged_original_examples() {
        let fixture = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/lint/literal_as_condition/original-parity.json"
        ));
        let cases: serde_json::Value = serde_json::from_str(fixture).unwrap();
        for case in cases.as_array().unwrap() {
            let source = SourceFile::from_bytes(
                "probe.rb",
                case["original"].as_str().unwrap().as_bytes().to_vec(),
            );
            let tree = crate::parse::parse_ruby(&source).unwrap();
            assert!(
                !tree.root_node().has_error(),
                "{} {}",
                case["id"],
                tree.root_node().to_sexp()
            );
        }
        crate::testutil::assert_original_spec_parity(&LiteralAsCondition, fixture);
    }
}
