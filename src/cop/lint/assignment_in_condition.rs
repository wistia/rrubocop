use std::collections::BTreeMap;
use tree_sitter::{Node, Tree};

use crate::cop::shared::{self, call_method_name, named_kids, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::{codemap::CodeMap, source::SourceFile};

pub struct AssignmentInCondition;
const SAFE: &str = "Use `==` if you meant to do a comparison or wrap the expression in parentheses to indicate you meant to assign in a condition.";
const UNSAFE: &str =
    "Use `==` if you meant to do a comparison or move the assignment up out of the condition.";
fn children(node: Node<'_>) -> Vec<Node<'_>> {
    named_kids(node)
        .into_iter()
        .filter(|child| !matches!(child.kind(), "comment" | "empty_statement"))
        .collect()
}
fn safe_allowed(config: &CopConfig) -> bool {
    !matches!(
        config.options.get("AllowSafeAssignment"),
        Some(serde_yml::Value::Bool(false) | serde_yml::Value::Null)
    )
}
fn setter(source: &SourceFile, node: Node<'_>) -> bool {
    shared::is_setter_call(node)
        || call_method_name(source, node).is_some_and(|name| {
            name.ends_with(b"=") && !matches!(name, b"==" | b"===" | b"!=" | b"<=" | b">=")
        })
}
fn assignment(node: Node<'_>) -> Option<Node<'_>> {
    if node.kind() != "assignment" {
        return None;
    }
    node.child_by_field_name("operator").or_else(|| {
        let mut cursor = node.walk();
        node.children(&mut cursor).find(|child| child.kind() == "=")
    })
}
fn numbered_body<'a>(source: &SourceFile, node: Node<'a>) -> Option<(Node<'a>, bool)> {
    let block = if node.kind() == "call" {
        node.child_by_field_name("block")?
    } else if node.kind() == "lambda" {
        node.child_by_field_name("body")?
    } else {
        return None;
    };
    let body = block.child_by_field_name("body").unwrap_or(block);
    let parameters = block
        .child_by_field_name("parameters")
        .or_else(|| node.child_by_field_name("parameters"));
    Some((
        body,
        parameters.is_none() && shared::numbered_parameter_arity(source, body) > 0,
    ))
}
fn collect<'a>(
    source: &SourceFile,
    node: Node<'a>,
    allow: bool,
    targets: &mut BTreeMap<(usize, usize, usize), Node<'a>>,
) {
    if let Some((body, numbered)) = numbered_body(source, node) {
        if numbered {
            collect(source, body, allow, targets);
        }
        return;
    }
    if matches!(node.kind(), "block" | "do_block") {
        return;
    }
    if node.kind() == "heredoc_beginning" {
        for interpolation in shared::heredoc_interpolations(source, node).unwrap_or_default() {
            collect(source, interpolation, allow, targets);
        }
        return;
    }
    if node.kind() == "unary"
        && node
            .child_by_field_name("operator")
            .is_some_and(|operator| node_bytes(source, operator) == b"defined?")
    {
        if let Some(operand) = node.child_by_field_name("operand") {
            let adjacent = node
                .child_by_field_name("operator")
                .is_some_and(|operator| operator.end_byte() == operand.start_byte());
            if operand.kind() == "parenthesized_statements" && adjacent {
                for child in children(operand) {
                    collect(source, child, allow, targets);
                }
            } else {
                collect(source, operand, allow, targets);
            }
        }
        return;
    }
    if matches!(node.kind(), "parenthesized_statements" | "interpolation") {
        let body = children(node);
        if allow && body.len() == 1 && body[0].kind() == "assignment" {
            return;
        }
    }
    if let Some(operator) = assignment(node) {
        targets.insert(
            (node.start_byte(), node.end_byte(), operator.start_byte()),
            node,
        );
    }
    if matches!(node.kind(), "call" | "element_reference") && !setter(source, node) {
        return;
    }
    if node.kind() == "unary"
        && node
            .child_by_field_name("operator")
            .is_none_or(|operator| node_bytes(source, operator) != b"defined?")
    {
        return;
    }
    if node.kind() == "binary"
        && node.child_by_field_name("operator").is_none_or(|operator| {
            !matches!(node_bytes(source, operator), b"&&" | b"||" | b"and" | b"or")
        })
    {
        return;
    }
    for child in children(node) {
        collect(source, child, allow, targets);
    }
}
fn render(source: &SourceFile, root: Node<'_>, targets: &[Node<'_>]) -> String {
    let mut inserts: BTreeMap<usize, (usize, usize)> = BTreeMap::new();
    for target in targets {
        inserts.entry(target.start_byte()).or_default().0 += 1;
        inserts.entry(target.end_byte()).or_default().1 += 1;
    }
    let mut result = Vec::new();
    let mut position = root.start_byte();
    for (offset, (open, close)) in inserts {
        result.extend_from_slice(&source.as_bytes()[position..offset]);
        result.extend(std::iter::repeat_n(b')', close));
        result.extend(std::iter::repeat_n(b'(', open));
        position = offset;
    }
    result.extend_from_slice(&source.as_bytes()[position..root.end_byte()]);
    String::from_utf8_lossy(&result).replace("\r\n", "\n")
}
impl Cop for AssignmentInCondition {
    fn name(&self) -> &'static str {
        "Lint/AssignmentInCondition"
    }
    fn redundant_disable_audit(&self) -> bool {
        false
    }
    fn default_severity(&self) -> Severity {
        Severity::Warning
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn safe_autocorrect(&self) -> bool {
        false
    }
    fn uses_source_phase(&self) -> bool {
        true
    }
    fn check_source(
        &self,
        source: &SourceFile,
        tree: &Tree,
        _map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let allow = safe_allowed(config);
        let mut targets = BTreeMap::new();
        let mut pending = vec![tree.root_node()];
        while let Some(node) = pending.pop() {
            pending.extend(children(node).into_iter().rev());
            if !matches!(
                node.kind(),
                "if" | "elsif"
                    | "unless"
                    | "conditional"
                    | "if_modifier"
                    | "unless_modifier"
                    | "while"
                    | "until"
                    | "while_modifier"
                    | "until_modifier"
            ) {
                continue;
            }
            if matches!(node.kind(), "while_modifier" | "until_modifier")
                && node
                    .child_by_field_name("body")
                    .is_some_and(|body| body.kind() == "begin")
            {
                continue;
            }
            if let Some(condition) = node.child_by_field_name("condition") {
                collect(source, condition, allow, &mut targets);
            }
        }
        let can_correct = allow
            && !config.autocorrect_disabled()
            && !tree.root_node().has_error()
            && std::str::from_utf8(source.as_bytes()).is_ok();
        for node in targets.values() {
            let operator = assignment(*node).unwrap();
            let (line, column) = source.offset_to_line_col(operator.start_byte());
            let mut diagnostic = self.diagnostic(
                source,
                line,
                column,
                if allow { SAFE } else { UNSAFE }.into(),
            );
            diagnostic.highlight_length = node_bytes(source, operator).len();
            diagnostic.correctable = can_correct;
            diagnostic.corrected = can_correct && corrections.is_some();
            diagnostics.push(diagnostic);
        }
        if !can_correct {
            return;
        }
        let initial = corrections.as_ref().map_or(0, |edits| edits.len());
        let nodes: Vec<_> = targets.values().copied().collect();
        for root in nodes.iter().copied().filter(|root| {
            !nodes.iter().any(|other| {
                other != root
                    && other.start_byte() <= root.start_byte()
                    && other.end_byte() >= root.end_byte()
            })
        }) {
            let group: Vec<_> = nodes
                .iter()
                .copied()
                .filter(|node| {
                    node.start_byte() >= root.start_byte() && node.end_byte() <= root.end_byte()
                })
                .collect();
            let replacement = render(source, root, &group);
            for node in group {
                let before = corrections.as_ref().map_or(0, |edits| edits.len());
                shared::push_replace(
                    &mut corrections,
                    root.start_byte(),
                    root.end_byte(),
                    replacement.clone(),
                    self.name(),
                );
                if let Some(edits) = corrections.as_mut() {
                    for edit in &mut edits[before..] {
                        edit.owner = Some(assignment(node).unwrap().start_byte());
                    }
                }
            }
        }
        if let Some(edits) = corrections {
            shared::normalize_corrected_crlf(source, self.name(), edits, initial);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(AssignmentInCondition, "cops/lint/assignment_in_condition");
    #[test]
    fn independent_original_states() {
        let fixture = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/lint/assignment_in_condition/original-independent.json"
        ));
        let cases: serde_json::Value = serde_json::from_str(fixture).unwrap();
        let mut native_errors = Vec::new();
        for case in cases.as_array().unwrap() {
            let source = SourceFile::from_bytes(
                "probe.rb",
                case["original"].as_str().unwrap().as_bytes().to_vec(),
            );
            let tree = crate::parse::parse_ruby(&source).unwrap();
            if tree.root_node().has_error() {
                native_errors.push(serde_json::json!({"id":case["id"],"source":case["original"],"tree":tree.root_node().to_sexp()}));
            }
            let config = CopConfig {
                options: case["config"]
                    .as_object()
                    .unwrap()
                    .iter()
                    .map(|(key, value)| (key.clone(), serde_yml::to_value(value).unwrap()))
                    .collect(),
                ..Default::default()
            };
            let mut actual = Vec::new();
            let mut corrections = Vec::new();
            AssignmentInCondition.check_source(
                &source,
                &tree,
                &CodeMap::default(),
                &config,
                &mut actual,
                Some(&mut corrections),
            );
            actual.sort_by_key(|diagnostic| (diagnostic.location.line, diagnostic.location.column));
            for key in ["correctable", "corrected"] {
                let expected: Vec<_> = case["ruby_diagnostics"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|diagnostic| diagnostic[key].as_bool().unwrap())
                    .collect();
                assert_eq!(
                    actual
                        .iter()
                        .map(|diagnostic| if key == "correctable" {
                            diagnostic.correctable
                        } else {
                            diagnostic.corrected
                        })
                        .collect::<Vec<_>>(),
                    expected,
                    "{} {key}",
                    case["id"]
                );
            }
            assert_eq!(
                case["ruby_preserves_ast_without_grouping"], true,
                "{}",
                case["id"]
            );
        }
        assert!(
            native_errors.is_empty(),
            "NATIVE_ERROR_COVERAGE {}",
            serde_json::to_string(&native_errors).unwrap()
        );
        crate::testutil::assert_original_spec_parity(&AssignmentInCondition, fixture);
    }
    #[test]
    fn unchanged_original_examples() {
        crate::testutil::assert_original_spec_parity(
            &AssignmentInCondition,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/lint/assignment_in_condition/original-parity.json"
            )),
        );
    }
}
