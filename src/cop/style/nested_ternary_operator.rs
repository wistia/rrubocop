use tree_sitter::{Node, Tree};

use crate::cop::shared::{self, named_kids, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::{codemap::CodeMap, source::SourceFile};

pub struct NestedTernaryOperator;
const MESSAGE: &str =
    "Ternary operators must not be nested. Prefer `if` or `else` constructs instead.";

fn descendants(node: Node<'_>) -> Vec<Node<'_>> {
    let mut result = Vec::new();
    let mut stack = named_kids(node).into_iter().rev().collect::<Vec<_>>();
    while let Some(child) = stack.pop() {
        stack.extend(named_kids(child).into_iter().rev());
        result.push(child);
    }
    result
}
fn token<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| child.kind() == kind)
}
fn whitespace(byte: &u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}
fn surrounding_space(source: &SourceFile, node: Node<'_>) -> (usize, usize) {
    let bytes = source.as_bytes();
    let (mut start, mut end) = (node.start_byte(), node.end_byte());
    while start > 0 && whitespace(&bytes[start - 1]) {
        start -= 1;
    }
    while bytes.get(end).is_some_and(whitespace) {
        end += 1;
    }
    (start, end)
}
fn line_literal(source: &SourceFile, node: Node<'_>) -> bool {
    node.kind() == "identifier"
        && node_bytes(source, node) == b"__LINE__"
        && !node
            .parent()
            .is_some_and(|parent| parent.child_by_field_name("method") == Some(node))
}
fn first_bare_argument(source: &SourceFile, node: Node<'_>) -> bool {
    node.parent().is_some_and(|parent| {
        parent.kind() == "argument_list"
            && node_bytes(source, parent).first() != Some(&b'(')
            && named_kids(parent)
                .into_iter()
                .find(|child| child.kind() != "comment")
                == Some(node)
    })
}
fn replacement(source: &SourceFile, node: Node<'_>, line_literals: &[usize]) -> Option<String> {
    if node.has_error() || first_bare_argument(source, node) {
        return None;
    }
    let question = token(node, "?")?;
    if line_literals
        .iter()
        .any(|offset| *offset > question.start_byte())
    {
        return None;
    }
    let colon = token(node, ":")?;
    let branch = node.child_by_field_name("consequence")?;
    let branch_text = node_bytes(source, branch);
    if branch_text.first() == Some(&b'(') && branch.kind() != "parenthesized_statements" {
        return None;
    }
    let branch_text = if branch_text.first() == Some(&b'(') {
        branch_text.strip_prefix(b"(")?.strip_suffix(b")")?
    } else {
        branch_text
    };
    let (q_start, q_end) = surrounding_space(source, question);
    let (c_start, c_end) = surrounding_space(source, colon);
    let mut edits = vec![
        (q_start, q_end, "\n".to_string()),
        (c_start, c_end, "\nelse\n".to_string()),
        (
            branch.start_byte(),
            branch.end_byte(),
            String::from_utf8(branch_text.to_vec()).ok()?,
        ),
    ];
    edits.sort_by_key(|edit| std::cmp::Reverse(edit.0));
    let mut text = String::from_utf8(node_bytes(source, node).to_vec()).ok()?;
    for (start, end, replacement) in edits {
        text.replace_range(
            start - node.start_byte()..end - node.start_byte(),
            &replacement,
        );
    }
    Some(format!("if {text}\nend").replace("\r\n", "\n"))
}
impl Cop for NestedTernaryOperator {
    fn name(&self) -> &'static str {
        "Style/NestedTernaryOperator"
    }
    fn supports_autocorrect(&self) -> bool {
        true
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
        let nodes = descendants(tree.root_node());
        let line_literals: Vec<_> = nodes
            .iter()
            .copied()
            .filter(|node| line_literal(source, *node))
            .map(|node| node.start_byte())
            .collect();
        let initial = corrections.as_ref().map_or(0, |edits| edits.len());
        for node in nodes
            .into_iter()
            .filter(|node| node.kind() == "conditional")
        {
            if std::iter::successors(node.parent(), |parent| parent.parent())
                .any(|parent| parent.kind() == "conditional")
            {
                continue;
            }
            let nested: Vec<_> = descendants(node)
                .into_iter()
                .filter(|node| node.kind() == "conditional")
                .collect();
            if nested.is_empty() {
                continue;
            }
            let replacement = replacement(source, node, &line_literals)
                .filter(|_| !config.autocorrect_disabled());
            for (index, nested) in nested.into_iter().enumerate() {
                let (line, column) = source.offset_to_line_col(nested.start_byte());
                let mut diagnostic = self.diagnostic(source, line, column, MESSAGE.into());
                diagnostic.highlight_length = String::from_utf8_lossy(node_bytes(source, nested))
                    .replace("\r\n", "\n")
                    .chars()
                    .count();
                diagnostic.correctable = index == 0 && replacement.is_some();
                if diagnostic.correctable {
                    let before = corrections.as_ref().map_or(0, |edits| edits.len());
                    diagnostic.corrected = shared::push_replace(
                        &mut corrections,
                        node.start_byte(),
                        node.end_byte(),
                        replacement.as_ref().unwrap().clone(),
                        self.name(),
                    );
                    if let Some(edits) = corrections.as_mut() {
                        for edit in &mut edits[before..] {
                            edit.owner = Some(nested.start_byte());
                        }
                    }
                }
                diagnostics.push(diagnostic);
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
    #[test]
    fn unchanged_original_examples() {
        crate::testutil::assert_original_spec_parity(
            &NestedTernaryOperator,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/nested_ternary_operator/original-parity.json"
            )),
        );
    }
    #[test]
    fn independent_original_states() {
        let mut cases: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/style/nested_ternary_operator/original-independent.json"
        )))
        .unwrap();
        for case in cases.as_array_mut().unwrap() {
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
            NestedTernaryOperator.check_source(
                &source,
                &tree,
                &CodeMap::default(),
                &config,
                &mut actual,
                Some(&mut corrections),
            );
            actual.sort_by_key(|diagnostic| (diagnostic.location.line, diagnostic.location.column));
            let expected: Vec<_> = case["ruby_diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .map(|diagnostic| {
                    case.get("rust_correctable")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or_else(|| diagnostic["correctable"].as_bool().unwrap())
                })
                .collect();
            assert_eq!(
                actual
                    .iter()
                    .map(|diagnostic| diagnostic.correctable)
                    .collect::<Vec<_>>(),
                expected,
                "{}",
                case["id"]
            );
            if case.get("rust_corrected").is_some() {
                case["ruby_corrected"] = case["rust_corrected"].clone();
            }
        }
        crate::testutil::assert_original_spec_parity(
            &NestedTernaryOperator,
            &serde_json::to_string(&cases).unwrap(),
        );
    }
}
