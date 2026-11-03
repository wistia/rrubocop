//! Style/RedundantParentheses.
mod correct;
mod semantics;
use crate::cop::shared::push_replace;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct RedundantParentheses;
impl Cop for RedundantParentheses {
    fn name(&self) -> &'static str {
        "Style/RedundantParentheses"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &[
            "parenthesized_statements",
            "parenthesized_pattern",
            "expression_reference_pattern",
        ]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(message) = semantics::message(source, node, config) else {
            return;
        };
        let start = node.start_byte() + usize::from(node.kind() == "expression_reference_pattern");
        let end = node.end_byte();
        let (line, column) = source.offset_to_line_col(start);
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            format!("Don't use parentheses around {message}."),
        );
        diagnostic.highlight_length = String::from_utf8_lossy(&source.as_bytes()[start..end])
            .chars()
            .count();
        if let Some(edits) = correct::edits(source, node, start, end) {
            for edit in edits {
                diagnostic.corrected |= push_replace(
                    &mut corrections,
                    edit.start,
                    edit.end,
                    edit.text,
                    self.name(),
                );
            }
        } else {
            diagnostic.correctable = false;
        }
        diagnostics.push(diagnostic);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn does_not_join_a_comma_into_a_line_comment() {
        let input = b"[(1 # retain\n), 2]\n";
        let diagnostics = crate::testutil::run_cop_full(&RedundantParentheses, input);
        assert_eq!(diagnostics.len(), 1);
        assert!(!diagnostics[0].correctable);
        let mut fixture: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/style/redundant_parentheses/intentional-safeguards.json"
        )))
        .unwrap();
        for case in fixture.as_array_mut().unwrap() {
            case["ruby_corrected"] = case["rust_corrected"].clone();
        }
        crate::testutil::assert_original_spec_parity(&RedundantParentheses, &fixture.to_string());
    }
    #[test]
    fn original_real_corpus_parent_roles() {
        crate::testutil::assert_original_spec_parity(
            &RedundantParentheses,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/redundant_parentheses/real-role-regressions.json"
            )),
        );
    }
    #[test]
    fn original_ruby_specs() {
        crate::testutil::assert_original_spec_parity(
            &RedundantParentheses,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/redundant_parentheses/parity.json"
            )),
        );
    }
}
