//! Layout/ArrayAlignment: original display geometry and nested-element correction.
use super::original_alignment::{self, Investigation, Item, display_column, line};
use crate::cop::shared::named_kids;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use tree_sitter::{Node, Tree};
pub struct ArrayAlignment;
fn items(node: Node<'_>) -> Vec<Item> {
    let mut result: Vec<Item> = Vec::new();
    let mut in_hash = false;
    for child in named_kids(node)
        .into_iter()
        .filter(|n| n.kind() != "comment")
    {
        let hash = matches!(child.kind(), "pair" | "hash_splat_argument");
        if hash && in_hash {
            result.last_mut().unwrap().end = child.end_byte();
        } else {
            result.push(Item {
                start: child.start_byte(),
                end: child.end_byte(),
            });
        }
        in_hash = hash;
    }
    result
}
fn parallel_assignment(node: Node<'_>) -> bool {
    node.parent().is_some_and(|parent| {
        parent.kind() == "assignment"
            && parent.child_by_field_name("left").is_some_and(|left| {
                matches!(
                    left.kind(),
                    "left_assignment_list" | "destructured_left_assignment"
                )
            })
    })
}
fn base_column(source: &SourceFile, node: Node<'_>, first: Item, config: &CopConfig) -> usize {
    if config.get_str("EnforcedStyle", "with_first_element") != "with_fixed_indentation" {
        return display_column(source, first.start);
    }
    let target = if matches!(node.kind(), "right_assignment_list" | "exceptions") {
        node.parent().unwrap_or(node)
    } else {
        node
    };
    let start = source
        .line_start(line(source, target.start_byte()))
        .unwrap_or(0);
    let prefix = &source.as_bytes()[start..target.start_byte()];
    prefix
        .iter()
        .position(|b| !matches!(b, b' ' | b'\t' | b'\r' | b'\x0b' | b'\x0c'))
        .unwrap_or(prefix.len())
        + config.get_usize("IndentationWidth", 2)
}

impl Cop for ArrayAlignment {
    fn name(&self) -> &'static str {
        "Layout/ArrayAlignment"
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
        let message = if config.get_str("EnforcedStyle", "with_first_element")
            == "with_fixed_indentation"
        {
            "Use one level of indentation for elements following the first line of a multi-line array."
        } else {
            "Align the elements of an array literal if they span more than one line."
        };
        for node in nodes.iter().copied().filter(|n| {
            matches!(
                n.kind(),
                "array" | "right_assignment_list" | "string_array" | "symbol_array" | "exceptions"
            )
        }) {
            if parallel_assignment(node) {
                continue;
            }
            let elements = items(node);
            if elements.len() < 2 {
                continue;
            }
            investigation.check(
                &elements,
                base_column(source, node, elements[0], config),
                message,
            );
        }
        investigation.finish();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_pipeline_preserves_multiline_correction_ownership() {
        use crate::cli::AutocorrectMode;
        use crate::config::{CopFilterSet, ResolvedConfig};
        use crate::cop::{CopRegistry, EnabledState};
        for (before, after) in [
            ("a = [\r\n  one,\r\n    two]\r\n", "a = [\n  one,\n  two]\n"),
            (
                "[one,\r\n    [two,\r\n       three]]\r\n",
                "[one,\n [two,\n  three]]\n",
            ),
        ] {
            for mode in [AutocorrectMode::Safe, AutocorrectMode::All] {
                let mut config = ResolvedConfig::empty();
                config.cop_configs.insert(
                    ArrayAlignment.name().into(),
                    CopConfig {
                        enabled: EnabledState::True,
                        options: [
                            ("EnforcedStyle".into(), "with_first_element".into()),
                            ("__RubocopVersion".into(), "1.65.1".into()),
                        ]
                        .into(),
                        ..CopConfig::default()
                    },
                );
                let registry = CopRegistry::default_registry();
                let filters = CopFilterSet::build(&config, &registry);
                let only = [ArrayAlignment.name().into()];
                let mut bytes = before.as_bytes().to_vec();
                crate::linter::lint_bytes_autocorrect(
                    std::path::Path::new("probe.rb"),
                    &mut bytes,
                    &config,
                    &registry,
                    &filters,
                    Some(&only),
                    &[],
                    mode,
                    false,
                )
                .unwrap();
                assert_eq!(bytes, after.as_bytes());
                let remaining = crate::linter::lint_bytes_autocorrect(
                    std::path::Path::new("probe.rb"),
                    &mut bytes,
                    &config,
                    &registry,
                    &filters,
                    Some(&only),
                    &[],
                    mode,
                    false,
                )
                .unwrap();
                assert!(remaining.is_empty());
                assert_eq!(bytes, after.as_bytes());
            }
        }
    }
    #[test]
    fn independent_original_states() {
        crate::testutil::assert_original_spec_parity(
            &ArrayAlignment,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/layout/array_alignment/independent-parity.json"
            )),
        );
    }
    #[test]
    fn original_literal_changes_are_not_reproduced() {
        let mut cases: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/layout/array_alignment/safeguards.json"
        )))
        .unwrap();
        for case in cases.as_array_mut().unwrap() {
            case["ruby_corrected"] = case["rust_corrected"].clone();
        }
        crate::testutil::assert_original_spec_parity(
            &ArrayAlignment,
            &serde_json::to_string(&cases).unwrap(),
        );
    }
    #[test]
    fn original_final_offense_states() {
        crate::testutil::assert_original_spec_parity(
            &ArrayAlignment,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/layout/array_alignment/final-parity.json"
            )),
        );
    }
    #[test]
    fn original_spec_states() {
        crate::testutil::assert_original_spec_parity(
            &ArrayAlignment,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/layout/array_alignment/original-parity.json"
            )),
        );
    }
}
