use tree_sitter::{Node, Tree};

use crate::cop::shared::{self, named_kids, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::{codemap::CodeMap, source::SourceFile};

pub struct EmptyInterpolation;

fn empty(node: Node<'_>) -> bool {
    node.kind() == "interpolation"
        && named_kids(node)
            .into_iter()
            .all(|child| matches!(child.kind(), "comment" | "empty_statement"))
}

fn removes_word(source: &SourceFile, node: Node<'_>) -> bool {
    let Some(word) = node
        .parent()
        .filter(|parent| matches!(parent.kind(), "bare_string" | "bare_symbol"))
    else {
        return false;
    };
    named_kids(word).into_iter().all(|child| {
        empty(child)
            || child.kind() == "escape_sequence"
                && matches!(node_bytes(source, child), b"\\\n" | b"\\\r\n")
    })
}

impl Cop for EmptyInterpolation {
    fn name(&self) -> &'static str {
        "Lint/EmptyInterpolation"
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
        let can_correct = !tree.root_node().has_error()
            && std::str::from_utf8(source.as_bytes()).is_ok()
            && !config.autocorrect_disabled();
        let initial = corrections.as_ref().map_or(0, |edits| edits.len());
        for node in nodes.into_iter().filter(|node| empty(*node)) {
            let (line, column) = source.offset_to_line_col(node.start_byte());
            let mut diagnostic =
                self.diagnostic(source, line, column, "Empty interpolation detected.".into());
            diagnostic.highlight_length = String::from_utf8_lossy(node_bytes(source, node))
                .replace("\r\n", "\n")
                .chars()
                .count();
            diagnostic.correctable = can_correct
                && !removes_word(source, node)
                && !(node_bytes(source, node).contains(&b'\n')
                    && lines.iter().any(|offset| *offset >= node.end_byte()));
            if diagnostic.correctable
                && let Some(edits) = corrections.as_mut()
            {
                edits.push(Correction {
                    start: node.start_byte(),
                    end: node.end_byte(),
                    replacement: String::new(),
                    cop_name: self.name(),
                    cop_index: 0,
                    owner: Some(node.start_byte()),
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
    fn replay(fixture: &str) {
        let mut cases: serde_json::Value = serde_json::from_str(fixture).unwrap();
        for case in cases.as_array_mut().unwrap() {
            assert!(case.get("ruby_cop_error").is_none(), "{}", case["id"]);
            if let Some(parser) = case.get("ruby_parser_diagnostics") {
                assert_eq!(parser, &serde_json::json!([]), "{}", case["id"]);
            }
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
            let mut edits = Vec::new();
            EmptyInterpolation.check_source(
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
                    let expected = case
                        .get("rust_correctable")
                        .and_then(serde_json::Value::as_bool)
                        .or_else(|| expected[key].as_bool())
                        .unwrap();
                    assert_eq!(actual, expected, "{} {key}", case["id"]);
                }
            }
            if let Some(exit) = case.get("native_final_exit") {
                assert_eq!(actual.is_empty(), exit == 0, "{}", case["id"]);
                let corrected = case
                    .get("rust_corrected")
                    .unwrap_or(&case["ruby_corrected"]);
                assert_eq!(&case["original"], corrected, "{}", case["id"]);
                assert!(edits.is_empty(), "{}", case["id"]);
            }
            if case.get("rust_corrected").is_some() {
                case["ruby_corrected"] = case["rust_corrected"].clone();
            }
        }
        crate::testutil::assert_original_spec_parity(
            &EmptyInterpolation,
            &serde_json::to_string(&cases).unwrap(),
        );
    }
    #[test]
    fn original_examples() {
        replay(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/lint/empty_interpolation/original-parity.json"
        )));
    }
    #[test]
    fn independent_states() {
        replay(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/lint/empty_interpolation/original-independent.json"
        )));
    }
    #[test]
    fn source_loss_guards_and_role_edges() {
        replay(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/lint/empty_interpolation/original-edges.json"
        )));
    }
    #[test]
    fn fresh_final_states_preserve_only_documented_guards() {
        replay(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/lint/empty_interpolation/final-states.json"
        )));
    }
}
