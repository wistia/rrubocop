use tree_sitter::Node;

use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Lint/EmptyEnsure — empty `ensure` body.
pub struct EmptyEnsure;

fn ensure_keyword(node: Node<'_>) -> Option<Node<'_>> {
    let mut cur = node.walk();
    node.children(&mut cur)
        .find(|ch| !ch.is_named() && ch.kind() == "ensure")
}

fn has_ensure_body(node: Node<'_>) -> bool {
    let mut cur = node.walk();
    node.named_children(&mut cur)
        .any(|child| !matches!(child.kind(), "comment" | "empty_statement"))
}

fn remove_ensure(cop_name: &'static str, kw: Node<'_>) -> Correction {
    Correction {
        start: kw.start_byte(),
        end: kw.end_byte(),
        replacement: String::new(),
        cop_name,
        cop_index: 0,
        owner: None,
    }
}

impl Cop for EmptyEnsure {
    fn name(&self) -> &'static str {
        "Lint/EmptyEnsure"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["ensure"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if has_ensure_body(node) {
            return;
        }
        let kw = ensure_keyword(node).unwrap_or(node);
        let (line, col) = source.offset_to_line_col(kw.start_byte());
        let mut diag = self.diagnostic(
            source,
            line,
            col,
            "Empty `ensure` block detected.".to_string(),
        );
        diag.highlight_length = kw.end_byte() - kw.start_byte();
        diag.correctable = !config.autocorrect_disabled();
        if !config.autocorrect_disabled()
            && let Some(corr) = corrections.as_mut()
        {
            let initial = corr.len();
            corr.push(remove_ensure(self.name(), kw));
            crate::cop::shared::normalize_corrected_crlf(source, self.name(), corr, initial);
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_examples() {
        crate::testutil::assert_original_spec_parity(
            &EmptyEnsure,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/lint/empty_ensure/original-parity.json"
            )),
        );
    }
    #[test]
    fn independent_sources_parse_and_preserve_correction_flags() {
        let cases: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/lint/empty_ensure/original-independent.json"
        )))
        .unwrap();
        for case in cases.as_array().unwrap() {
            let source = SourceFile::from_bytes(
                "probe.rb",
                case["original"].as_str().unwrap().as_bytes().to_vec(),
            );
            let tree = crate::parse::parse_ruby(&source).unwrap();
            assert!(!tree.root_node().has_error(), "{}", case["id"]);
            let mut nodes = vec![tree.root_node()];
            let mut actual = Vec::new();
            let mut edits = Vec::new();
            while let Some(node) = nodes.pop() {
                if node.kind() == "ensure" {
                    EmptyEnsure.check_node(
                        &source,
                        node,
                        &CopConfig::default(),
                        &mut actual,
                        Some(&mut edits),
                    );
                }
                nodes.extend(crate::cop::shared::named_kids(node));
            }
            let expected = case["ruby_diagnostics"].as_array().unwrap();
            assert_eq!(actual.len(), expected.len(), "{}", case["id"]);
            for (actual, expected) in actual.iter().zip(expected) {
                assert_eq!(
                    actual.correctable,
                    expected["correctable"].as_bool().unwrap(),
                    "{}",
                    case["id"]
                );
                assert_eq!(
                    actual.corrected,
                    expected["corrected"].as_bool().unwrap(),
                    "{}",
                    case["id"]
                );
            }
        }
    }
    #[test]
    fn independent_states() {
        crate::testutil::assert_original_spec_parity(
            &EmptyEnsure,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/lint/empty_ensure/original-independent.json"
            )),
        );
    }
    #[test]
    fn fresh_final_states_are_clean_and_stable() {
        let fixture = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/lint/empty_ensure/final-states.json"
        ));
        let cases: serde_json::Value = serde_json::from_str(fixture).unwrap();
        for case in cases.as_array().unwrap() {
            let source = SourceFile::from_bytes(
                "probe.rb",
                case["original"].as_str().unwrap().as_bytes().to_vec(),
            );
            assert!(
                !crate::parse::parse_ruby(&source)
                    .unwrap()
                    .root_node()
                    .has_error(),
                "{}",
                case["id"]
            );
            assert_eq!(
                case["ruby_parser_diagnostics"],
                serde_json::json!([]),
                "{}",
                case["id"]
            );
            assert_eq!(
                case["ruby_diagnostics"],
                serde_json::json!([]),
                "{}",
                case["id"]
            );
            assert_eq!(case["original"], case["ruby_corrected"], "{}", case["id"]);
        }
        crate::testutil::assert_original_spec_parity(&EmptyEnsure, fixture);
    }
    #[test]
    fn disabled_correction_retains_original_warning_without_edit() {
        let source = SourceFile::from_bytes("probe.rb", b"begin;work;ensure;end\n".to_vec());
        let tree = crate::parse::parse_ruby(&source).unwrap();
        let ensure = crate::cop::shared::named_kids(tree.root_node())
            .into_iter()
            .flat_map(crate::cop::shared::named_kids)
            .find(|node| node.kind() == "ensure")
            .unwrap();
        for value in [
            serde_yml::Value::Bool(false),
            serde_yml::Value::String("disabled".into()),
        ] {
            let mut config = CopConfig::default();
            config.options.insert("AutoCorrect".into(), value);
            let mut diagnostics = Vec::new();
            let mut edits = Vec::new();
            EmptyEnsure.check_node(&source, ensure, &config, &mut diagnostics, Some(&mut edits));
            assert_eq!(diagnostics.len(), 1);
            assert!(!diagnostics[0].correctable);
            assert!(!diagnostics[0].corrected);
            assert!(edits.is_empty());
        }
    }
}
