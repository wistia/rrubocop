use tree_sitter::Node;

use crate::cop::shared::{float_literal_span, node_text};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

/// Lint/FloatOutOfRange — float overflow and underflow detection.
pub struct FloatOutOfRange;

impl Cop for FloatOutOfRange {
    fn name(&self) -> &'static str {
        "Lint/FloatOutOfRange"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["float"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let Some((start, end)) = float_literal_span(source, node) else {
            return;
        };
        let Ok(value) = node_text(source, node).replace('_', "").parse::<f64>() else {
            return;
        };
        let literal = &source.as_bytes()[start..end];
        if !value.is_infinite()
            && !(value == 0.0 && literal.iter().any(|byte| matches!(byte, b'1'..=b'9')))
        {
            return;
        }
        let (line, column) = source.offset_to_line_col(start);
        let mut diagnostic = self.diagnostic(source, line, column, "Float out of range.".into());
        diagnostic.highlight_length = String::from_utf8_lossy(literal)
            .replace("\r\n", "\n")
            .chars()
            .count();
        diagnostics.push(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn replay(fixture: &str) {
        let cases: serde_json::Value = serde_json::from_str(fixture).unwrap();
        for case in cases.as_array().unwrap() {
            assert!(
                !case["ruby_parser_diagnostics"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|diagnostic| matches!(
                        diagnostic["level"].as_str(),
                        Some("error" | "fatal")
                    )),
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
            let config = CopConfig {
                options: case["config"]
                    .as_object()
                    .unwrap()
                    .iter()
                    .map(|(key, value)| (key.clone(), serde_yml::to_value(value).unwrap()))
                    .collect(),
                ..Default::default()
            };
            let mut diagnostics = Vec::new();
            let mut corrections = Vec::new();
            crate::cop::walker::BatchedWalker::new(vec![&FloatOutOfRange], vec![&config]).walk(
                &source,
                tree.root_node(),
                &mut diagnostics,
                Some(&mut corrections),
            );
            diagnostics
                .sort_by_key(|diagnostic| (diagnostic.location.line, diagnostic.location.column));
            let expected = case["ruby_diagnostics"].as_array().unwrap();
            assert_eq!(diagnostics.len(), expected.len(), "{}", case["id"]);
            for (actual, expected) in diagnostics.iter().zip(expected) {
                assert!(!actual.correctable && !actual.corrected, "{}", case["id"]);
                assert_eq!(expected["correctable"], false, "{}", case["id"]);
                assert_eq!(expected["corrected"], false, "{}", case["id"]);
            }
            assert!(corrections.is_empty(), "{}", case["id"]);
            assert_eq!(case["original"], case["ruby_corrected"], "{}", case["id"]);
        }
        crate::testutil::assert_original_spec_parity(&FloatOutOfRange, fixture);
    }
    #[test]
    fn original_examples() {
        replay(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/lint/float_out_of_range/original-parity.json"
        )));
    }
    #[test]
    fn independent_original_states() {
        replay(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/lint/float_out_of_range/original-independent.json"
        )));
    }
}
