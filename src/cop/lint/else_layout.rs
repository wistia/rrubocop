use tree_sitter::{Node, Tree};

use crate::cop::shared::{self, named_kids, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::{codemap::CodeMap, source::SourceFile};

pub struct ElseLayout;
const MESSAGE: &str = "Odd `else` layout detected. Did you mean to use `elsif`?";

fn statements(node: Node<'_>) -> Vec<Node<'_>> {
    named_kids(node)
        .into_iter()
        .filter(|child| !matches!(child.kind(), "comment" | "empty_statement" | "heredoc_body"))
        .collect()
}

fn candidate(mut conditional: Node<'_>) -> Option<(Node<'_>, Node<'_>, Node<'_>)> {
    loop {
        let alternative = conditional.child_by_field_name("alternative")?;
        if alternative.kind() == "elsif" {
            conditional = alternative;
            continue;
        }
        if alternative.kind() != "else" {
            return None;
        }
        let body = statements(alternative);
        let first = *body.first()?;
        let first = if body.len() == 1 && first.kind() == "parenthesized_statements" {
            *statements(first).first()?
        } else {
            first
        };
        let mut cursor = alternative.walk();
        let keyword = alternative
            .children(&mut cursor)
            .find(|child| child.kind() == "else")?;
        return (first.start_position().row == keyword.start_position().row).then_some((
            conditional,
            keyword,
            first,
        ));
    }
}

impl Cop for ElseLayout {
    fn name(&self) -> &'static str {
        "Lint/ElseLayout"
    }
    fn default_severity(&self) -> Severity {
        Severity::Warning
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
        let mut nodes = Vec::new();
        let mut pending = vec![tree.root_node()];
        while let Some(node) = pending.pop() {
            pending.extend(named_kids(node).into_iter().rev());
            nodes.push(node);
        }
        let lines: Vec<_> = nodes
            .iter()
            .filter(|node| shared::is_line_literal(source, **node))
            .map(Node::start_byte)
            .collect();
        let valid = !tree.root_node().has_error()
            && std::str::from_utf8(source.as_bytes()).is_ok()
            && !config.autocorrect_disabled();
        let initial = corrections.as_ref().map_or(0, |edits| edits.len());
        for node in nodes.into_iter().filter(|node| {
            matches!(node.kind(), "if" | "unless")
                && node.start_position().row != node.end_position().row
        }) {
            let Some((conditional, keyword, first)) = candidate(node) else {
                continue;
            };
            let (line, column) = source.offset_to_line_col(first.start_byte());
            let mut diagnostic = self.diagnostic(source, line, column, MESSAGE.into());
            diagnostic.highlight_length = String::from_utf8_lossy(node_bytes(source, first))
                .replace("\r\n", "\n")
                .chars()
                .count();
            let prefix = &source.as_bytes()[keyword.end_byte()..first.start_byte()];
            let indentation = source
                .offset_to_line_col(conditional.start_byte())
                .1
                .checked_add(config.get_usize("IndentationWidth", 2));
            diagnostic.correctable = valid
                && indentation.is_some()
                && prefix
                    .iter()
                    .all(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n' | 0x0b | 0x0c | b';'))
                && !lines.iter().any(|offset| *offset >= keyword.end_byte());
            if diagnostic.correctable
                && let Some(edits) = corrections.as_mut()
            {
                edits.push(Correction {
                    start: keyword.end_byte(),
                    end: first.start_byte(),
                    replacement: format!("\n{}", " ".repeat(indentation.unwrap())),
                    cop_name: self.name(),
                    cop_index: 0,
                    owner: Some(first.start_byte()),
                });
                diagnostic.corrected = true;
            }
            diagnostics.push(diagnostic);
        }
        if let Some(edits) = corrections {
            shared::normalize_corrected_crlf(source, self.name(), edits, initial);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(ElseLayout, "cops/lint/else_layout");
    fn replay(fixture: &str) {
        let mut cases: serde_json::Value = serde_json::from_str(fixture).unwrap();
        for case in cases.as_array_mut().unwrap() {
            assert_eq!(
                case["ruby_parser_diagnostics"],
                serde_json::json!([]),
                "{}",
                case["id"]
            );
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
            case["config"]["IndentationWidth"] = case["effective_indentation_width"].clone();
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
            let mut edits = Vec::new();
            ElseLayout.check_source(
                &source,
                &tree,
                &CodeMap::default(),
                &config,
                &mut actual,
                Some(&mut edits),
            );
            actual.sort_by_key(|diagnostic| (diagnostic.location.line, diagnostic.location.column));
            let expected = case["ruby_diagnostics"].as_array().unwrap();
            assert_eq!(actual.len(), expected.len(), "{}", case["id"]);
            for (actual, expected) in actual.iter().zip(expected) {
                for (key, actual) in [
                    ("correctable", actual.correctable),
                    ("corrected", actual.corrected),
                ] {
                    assert_eq!(
                        actual,
                        case.get("rust_correctable")
                            .and_then(serde_json::Value::as_bool)
                            .or_else(|| expected[key].as_bool())
                            .unwrap(),
                        "{} {key}",
                        case["id"]
                    );
                }
            }
            if case.get("rust_corrected").is_some() {
                case["ruby_corrected"] = case["rust_corrected"].clone();
            }
        }
        crate::testutil::assert_original_spec_parity(
            &ElseLayout,
            &serde_json::to_string(&cases).unwrap(),
        );
    }
    #[test]
    fn original_examples() {
        replay(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/lint/else_layout/original-parity.json"
        )));
    }
    #[test]
    fn independent_original_states() {
        replay(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/lint/else_layout/original-independent.json"
        )));
    }
}
