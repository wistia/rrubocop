//! Layout/ParameterAlignment: original method-only parameter geometry.
use super::original_alignment::{self, Investigation, Item, display_column, line};
use crate::cop::shared::named_kids;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use tree_sitter::{Node, Tree};
pub struct ParameterAlignment;
fn base(source: &SourceFile, node: Node<'_>, first: Item, config: &CopConfig) -> usize {
    if config.get_str("EnforcedStyle", "with_first_parameter") != "with_fixed_indentation" {
        return display_column(source, first.start);
    }
    let start = source
        .line_start(line(source, node.start_byte()))
        .unwrap_or(0);
    let prefix = &source.as_bytes()[start..node.start_byte()];
    prefix
        .iter()
        .position(|b| !matches!(b, b' ' | b'\t' | b'\r' | b'\x0b' | b'\x0c'))
        .unwrap_or(prefix.len())
        + config.get_usize("IndentationWidth", 2)
}
impl Cop for ParameterAlignment {
    fn name(&self) -> &'static str {
        "Layout/ParameterAlignment"
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
        corrections: Option<&mut Vec<Correction>>,
    ) {
        let nodes = original_alignment::nodes(tree);
        let mut investigation = Investigation::new(self, source, &nodes, diagnostics, corrections);
        let message = if config.get_str("EnforcedStyle", "with_first_parameter")
            == "with_fixed_indentation"
        {
            "Use one level of indentation for parameters following the first line of a multi-line method definition."
        } else {
            "Align the parameters of a method definition if they span more than one line."
        };
        for node in nodes
            .iter()
            .copied()
            .filter(|n| matches!(n.kind(), "method" | "singleton_method"))
        {
            let Some(parameters) = node.child_by_field_name("parameters") else {
                continue;
            };
            let items: Vec<_> = named_kids(parameters)
                .into_iter()
                .filter(|n| n.kind() != "comment")
                .map(|n| Item {
                    start: n.start_byte(),
                    end: n.end_byte(),
                })
                .collect();
            if items.len() < 2 {
                continue;
            }
            investigation.check(&items, base(source, node, items[0], config), message);
        }
        investigation.finish();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_original_states() {
        crate::testutil::assert_original_spec_parity(
            &ParameterAlignment,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/layout/parameter_alignment/independent-parity.json"
            )),
        );
    }
    #[test]
    fn original_literal_changes_are_not_reproduced() {
        let mut cases: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/layout/parameter_alignment/safeguards.json"
        )))
        .unwrap();
        for case in cases.as_array_mut().unwrap() {
            case["ruby_corrected"] = case["rust_corrected"].clone();
        }
        crate::testutil::assert_original_spec_parity(
            &ParameterAlignment,
            &serde_json::to_string(&cases).unwrap(),
        );
    }
    #[test]
    fn original_final_offense_states() {
        crate::testutil::assert_original_spec_parity(
            &ParameterAlignment,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/layout/parameter_alignment/final-parity.json"
            )),
        );
    }
    #[test]
    fn original_spec_states() {
        crate::testutil::assert_original_spec_parity(
            &ParameterAlignment,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/layout/parameter_alignment/original-parity.json"
            )),
        );
    }
}
