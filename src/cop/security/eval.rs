//! Security/Eval: pinned receiver and recursive-literal rules.
use tree_sitter::Node;

use crate::cop::shared::{
    call_method_name, call_receiver, named_kids, node_bytes, semantic_argument_nodes,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct Eval;
fn children(node: Node<'_>) -> Vec<Node<'_>> {
    named_kids(node)
        .into_iter()
        .filter(|node| node.kind() != "comment")
        .collect()
}
fn operator<'a>(source: &'a SourceFile, node: Node<'_>) -> &'a [u8] {
    node.child_by_field_name("operator")
        .map(|node| node_bytes(source, node))
        .unwrap_or_default()
}
fn implicit_binding(source: &SourceFile, node: Node<'_>) -> bool {
    match node.kind() {
        "identifier" => {
            node_bytes(source, node) == b"binding"
                && crate::cop::ported::bare_call_identifier(source, node)
        }
        "call" => {
            call_method_name(source, node) == Some(b"binding")
                && call_receiver(node).is_none()
                && semantic_argument_nodes(node).is_empty()
                && node.child_by_field_name("block").is_none()
        }
        _ => false,
    }
}
fn string_argument(source: &SourceFile, node: Node<'_>) -> bool {
    match node.kind() {
        "string" | "chained_string" | "character" | "file" => true,
        "identifier" => node_bytes(source, node) == b"__FILE__",
        "heredoc_beginning" => !matches!(
            node_bytes(source, node),
            [b'<', b'<', b'`', ..] | [b'<', b'<', b'-' | b'~', b'`', ..]
        ),
        _ => false,
    }
}
fn literal_interpolations(source: &SourceFile, node: Node<'_>) -> bool {
    if node.kind() == "chained_string" {
        return children(node)
            .into_iter()
            .all(|child| original_recursive_literal(source, child, false));
    }
    let mut stack = if node.kind() == "heredoc_beginning" {
        let Some((start, end)) = crate::cop::ported::heredoc_span(source, node) else {
            return false;
        };
        let root = std::iter::successors(Some(node), |node| node.parent())
            .last()
            .unwrap();
        let mut result = Vec::new();
        let mut stack = vec![root];
        while let Some(child) = stack.pop() {
            if child.kind() == "interpolation"
                && child.start_byte() >= start
                && child.end_byte() <= end
            {
                result.push(child);
            } else {
                stack.extend(children(child));
            }
        }
        result
    } else {
        children(node)
    };
    while let Some(child) = stack.pop() {
        if child.kind() == "interpolation"
            && !children(child)
                .into_iter()
                .all(|expression| original_recursive_literal(source, expression, false))
        {
            return false;
        }
    }
    true
}
fn literal_method(method: &[u8]) -> bool {
    matches!(
        method,
        b"==" | b"===" | b"!=" | b"<=" | b">=" | b">" | b"<" | b"*" | b"!" | b"<=>"
    )
}
fn original_recursive_literal(source: &SourceFile, node: Node<'_>, conditional: bool) -> bool {
    match node.kind() {
        "integer" | "float" | "rational" | "complex" | "true" | "false" | "nil" | "character"
        | "file" | "line" | "simple_symbol" | "hash_key_symbol" => true,
        "identifier" => matches!(node_bytes(source, node), b"__FILE__" | b"__LINE__"),
        "string" | "chained_string" | "delimited_symbol" | "bare_string" | "bare_symbol"
        | "subshell" | "heredoc_beginning" => literal_interpolations(source, node),
        "regex" => !conditional && literal_interpolations(source, node),
        "range" => {
            !conditional
                && children(node)
                    .into_iter()
                    .all(|child| original_recursive_literal(source, child, false))
        }
        "array" | "hash" | "string_array" | "symbol_array" => children(node)
            .into_iter()
            .all(|child| original_recursive_literal(source, child, false)),
        "pair" => {
            node.child_by_field_name("value").is_some()
                && children(node)
                    .into_iter()
                    .all(|child| original_recursive_literal(source, child, false))
        }
        "parenthesized_statements" | "interpolation" => {
            let body = children(node);
            let conditional = conditional && body.len() == 1;
            body.into_iter()
                .all(|child| original_recursive_literal(source, child, conditional))
        }
        "binary" => {
            let logical = matches!(operator(source, node), b"&&" | b"||" | b"and" | b"or");
            (logical || literal_method(operator(source, node)))
                && ["left", "right"].into_iter().all(|field| {
                    node.child_by_field_name(field).is_some_and(|child| {
                        original_recursive_literal(source, child, conditional && logical)
                    })
                })
        }
        "unary" => {
            let Some(operand) = node.child_by_field_name("operand") else {
                return false;
            };
            match operator(source, node) {
                b"+" | b"-" => {
                    matches!(operand.kind(), "integer" | "float" | "rational" | "complex")
                }
                b"!" | b"not" => original_recursive_literal(source, operand, true),
                _ => false,
            }
        }
        "call" => {
            node.child_by_field_name("block").is_none()
                && operator(source, node) != b"&."
                && call_method_name(source, node).is_some_and(literal_method)
                && call_receiver(node)
                    .is_some_and(|receiver| original_recursive_literal(source, receiver, false))
                && semantic_argument_nodes(node)
                    .into_iter()
                    .all(|argument| original_recursive_literal(source, argument, false))
        }
        _ => false,
    }
}
impl Cop for Eval {
    fn name(&self) -> &'static str {
        "Security/Eval"
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"eval"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if call_method_name(source, node) != Some(b"eval")
            || operator(source, node) == b"&."
            || call_receiver(node).is_some_and(|receiver| !implicit_binding(source, receiver))
        {
            return;
        }
        let Some(code) = semantic_argument_nodes(node).first().copied() else {
            return;
        };
        if string_argument(source, code) && original_recursive_literal(source, code, false) {
            return;
        }
        let Some(selector) = node.child_by_field_name("method") else {
            return;
        };
        let (line, column) = source.offset_to_line_col(selector.start_byte());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "The use of `eval` is a serious security risk.".into(),
        );
        diagnostic.highlight_length = 4;
        diagnostics.push(diagnostic);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(Eval, "cops/security/eval");
    #[test]
    fn independent_original_states() {
        let fixture = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/security/eval/original-independent.json"
        ));
        let cases: serde_json::Value = serde_json::from_str(fixture).unwrap();
        let mut errors = Vec::new();
        for case in cases.as_array().unwrap() {
            let source = SourceFile::from_bytes(
                "probe.rb",
                case["original"].as_str().unwrap().as_bytes().to_vec(),
            );
            let tree = crate::parse::parse_ruby(&source).unwrap();
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
            crate::cop::walker::BatchedWalker::new(vec![&Eval], vec![&config]).walk(
                &source,
                tree.root_node(),
                &mut actual,
                Some(&mut corrections),
            );
            actual.sort_by_key(|diagnostic| (diagnostic.location.line, diagnostic.location.column));
            assert!(
                corrections.is_empty()
                    && actual
                        .iter()
                        .all(|diagnostic| !diagnostic.correctable && !diagnostic.corrected)
            );
            let expected: Vec<_> = case["ruby_diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .map(|diagnostic| {
                    crate::diagnostic::Severity::from_str(diagnostic["severity"].as_str().unwrap())
                        .unwrap()
                })
                .collect();
            assert_eq!(
                actual
                    .iter()
                    .map(|diagnostic| diagnostic.severity)
                    .collect::<Vec<_>>(),
                expected,
                "{}",
                case["id"]
            );
            if tree.root_node().has_error() {
                errors.push(serde_json::json!({"id":case["id"],"source":case["original"],"tree":tree.root_node().to_sexp()}));
            }
        }
        assert!(
            errors.is_empty(),
            "NATIVE_ERROR_COVERAGE {}",
            serde_json::to_string(&errors).unwrap()
        );
        crate::testutil::assert_original_spec_parity(&Eval, fixture);
    }
    #[test]
    fn unchanged_original_examples() {
        crate::testutil::assert_original_spec_parity(
            &Eval,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/security/eval/original-parity.json"
            )),
        );
    }
}
