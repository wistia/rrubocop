//! Layout/ArgumentAlignment: original logical arguments and layout compatibility.
mod calls;
use crate::cop::layout::original_alignment::{self, Investigation, display_column, line};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;
use tree_sitter::Tree;
pub struct ArgumentAlignment;
fn separator_style(value: &serde_yml::Value) -> bool {
    match value {
        serde_yml::Value::String(style) => style.contains("separator"),
        serde_yml::Value::Sequence(styles) => styles
            .iter()
            .any(|style| style.as_str() == Some("separator")),
        _ => false,
    }
}
fn incompatible(config: &CopConfig) -> bool {
    config.get_str("EnforcedStyle", "with_first_argument") == "with_first_argument"
        && config.get_bool("HashAlignmentEnabled", false)
        && [
            "HashAlignmentEnforcedColonStyle",
            "HashAlignmentEnforcedHashRocketStyle",
        ]
        .into_iter()
        .any(|name| config.options.get(name).is_some_and(separator_style))
}
fn fixed_base(source: &SourceFile, selector: usize, width: usize) -> usize {
    let start = source.line_start(line(source, selector)).unwrap_or(0);
    let prefix = &source.as_bytes()[start..selector];
    prefix
        .iter()
        .position(|byte| !matches!(byte, b' ' | b'\t' | b'\r' | b'\x0b' | b'\x0c'))
        .unwrap_or(prefix.len())
        + width
}
impl Cop for ArgumentAlignment {
    fn name(&self) -> &'static str {
        "Layout/ArgumentAlignment"
    }
    fn redundant_disable_audit(&self) -> bool {
        false
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
        if incompatible(config) {
            return;
        }
        let nodes = original_alignment::nodes(tree);
        let fixed =
            config.get_str("EnforcedStyle", "with_first_argument") == "with_fixed_indentation";
        let message = if fixed {
            "Use one level of indentation for arguments following the first line of a multi-line method call."
        } else {
            "Align the arguments of a method call if they span more than one line."
        };
        let calls = calls::collect(source, &nodes, config);
        let mut investigation = Investigation::new(self, source, &nodes, diagnostics, corrections);
        for call in calls {
            let Some(items) = call.items(fixed) else {
                continue;
            };
            let base = if fixed || items.is_empty() {
                fixed_base(
                    source,
                    call.selector,
                    config.get_usize("IndentationWidth", 2),
                )
            } else {
                display_column(source, items[0].start)
            };
            investigation.check(&items, base, message);
        }
        investigation.finish();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_spec_states() {
        crate::testutil::assert_original_spec_parity(
            &ArgumentAlignment,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/layout/argument_alignment/original-parity.json"
            )),
        );
    }
    #[test]
    fn original_eligible_call_ownership_and_flattening() {
        let rows: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/layout/argument_alignment/original-call-roles.json"
        )))
        .unwrap();
        let mut compared = 0;
        let mut unsupported = Vec::new();
        let mut original_invalid = 0;
        for row in rows.as_array().unwrap() {
            if row["parser_diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|diagnostic| matches!(diagnostic["level"].as_str(), Some("error" | "fatal")))
            {
                original_invalid += 1;
                continue;
            }
            let source = SourceFile::from_bytes(
                "probe.rb",
                row["source"].as_str().unwrap().as_bytes().to_vec(),
            );
            let tree = crate::parse::parse_ruby(&source).unwrap();
            if tree.root_node().has_error() {
                unsupported.push(serde_json::json!({"id":row["id"],"source":row["source"],"ruby_version":row.get("ruby_version").cloned().unwrap_or(serde_json::json!(3.4))}));
                continue;
            }
            compared += 1;
            let mut config = CopConfig::default();
            config.options.insert(
                "TargetRubyVersion".into(),
                serde_yml::to_value(
                    row.get("ruby_version")
                        .and_then(serde_json::Value::as_f64)
                        .unwrap_or(3.4),
                )
                .unwrap(),
            );
            let nodes = original_alignment::nodes(&tree);
            let calls = calls::collect(&source, &nodes, &config);
            let text = |item: crate::cop::layout::original_alignment::Item| {
                String::from_utf8_lossy(&source.as_bytes()[item.start..item.end])
                    .replace("\r\n", "\n")
            };
            let actual:Vec<_>=calls.into_iter().filter(|call|call.items(false).is_some()).map(|call| {
                let target=source.offset_to_line_col(call.selector);
                let first:Vec<_>=call.items(false).unwrap().into_iter().map(|item|serde_json::json!({"source":text(item),"position":[source.offset_to_line_col(item.start).0,source.offset_to_line_col(item.start).1]})).collect();
                let fixed:Vec<_>=call.items(true).unwrap().into_iter().map(|item|serde_json::json!({"source":text(item),"position":[source.offset_to_line_col(item.start).0,source.offset_to_line_col(item.start).1]})).collect();
                serde_json::json!({"method":call.method,"type":if call.csend {"csend"} else {"send"},"target":[target.0,target.1],"first":first,"fixed":fixed})
            }).collect();
            let expected:Vec<_>=row["calls"].as_array().unwrap().iter().filter(|call|call["eligible"].as_bool()==Some(true)).map(|call| {
                let items=|style:&str|call["flattened"][style].as_array().unwrap().iter().map(|item|serde_json::json!({"source":item["source"],"position":item["position"]})).collect::<Vec<_>>();
                serde_json::json!({"method":call["method"],"type":call["type"],"target":call["target"],"first":items("with_first_argument"),"fixed":items("with_fixed_indentation")})
            }).collect();
            assert_eq!(
                actual,
                expected,
                "{} {} {}",
                row["id"],
                row["source"],
                tree.root_node().to_sexp()
            );
        }
        eprintln!(
            "ARGUMENT_ROLE_COVERAGE {}",
            serde_json::json!({"compared":compared,"original_invalid":original_invalid,"native_error_sources":unsupported})
        );
    }
    #[test]
    fn independent_original_states() {
        crate::testutil::assert_original_spec_parity(
            &ArgumentAlignment,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/layout/argument_alignment/independent-parity.json"
            )),
        );
    }
    #[test]
    fn original_crlf_states() {
        crate::testutil::assert_original_spec_parity(
            &ArgumentAlignment,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/layout/argument_alignment/crlf-parity.json"
            )),
        );
    }
    #[test]
    fn final_native_sources_match_fresh_original_states() {
        crate::testutil::assert_original_spec_parity(
            &ArgumentAlignment,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/layout/argument_alignment/final-states.json"
            )),
        );
    }
    #[test]
    fn original_literal_changes_are_not_reproduced() {
        let mut cases: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cops/layout/argument_alignment/safeguards.json"
        )))
        .unwrap();
        for case in cases.as_array_mut().unwrap() {
            case["ruby_corrected"] = case["rust_corrected"].clone();
        }
        crate::testutil::assert_original_spec_parity(
            &ArgumentAlignment,
            &serde_json::to_string(&cases).unwrap(),
        );
    }
    crate::cop_fixture_tests!(ArgumentAlignment, "cops/layout/argument_alignment");

    #[test]
    fn no_offense_command_fixture() {
        crate::testutil::assert_cop_no_offenses_full(
            &ArgumentAlignment,
            include_bytes!(
                "../../../tests/fixtures/cops/layout/argument_alignment/no_offense_command.rb"
            ),
        );
    }

    #[test]
    fn no_offense_kwargs_fixture() {
        crate::testutil::assert_cop_no_offenses_full(
            &ArgumentAlignment,
            include_bytes!(
                "../../../tests/fixtures/cops/layout/argument_alignment/no_offense_kwargs.rb"
            ),
        );
    }

    #[test]
    fn no_offense_trailing_kwargs_fixture() {
        crate::testutil::assert_cop_no_offenses_full(
            &ArgumentAlignment,
            include_bytes!(
                "../../../tests/fixtures/cops/layout/argument_alignment/no_offense_trailing_kwargs.rb"
            ),
        );
    }
}
