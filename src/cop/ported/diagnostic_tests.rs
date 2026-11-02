use super::*;
use crate::cli::AutocorrectMode;
use crate::config::{CopFilterSet, ResolvedConfig};
use crate::cop::EnabledState;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct RangeCase {
    cop: String,
    source: String,
    path: String,
    config: HashMap<String, serde_yml::Value>,
    expected: Vec<RangeExpected>,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
struct RangeExpected {
    line: usize,
    column: usize,
    message: String,
    length: usize,
    severity: String,
}

#[test]
fn audit_ranges_and_severities_match_the_pinned_ruby_reference() {
    compare_cases(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/diagnostic-range-parity.json"
    )));
}

#[test]
fn unicode_crlf_assignment_and_block_ranges_match_pinned_controls() {
    compare_cases(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/diagnostic-range-controls.json"
    )));
}

fn compare_cases(json: &str) {
    let registry = CopRegistry::default_registry();
    for case in serde_json::from_str::<Vec<RangeCase>>(json).unwrap() {
        let mut expected = case.expected.clone();
        let mut actual = inspect(&registry, &case);
        expected.sort_by_key(key);
        actual.sort_by_key(key);
        assert_eq!(actual, expected, "{}: {}", case.cop, case.source);
    }
}

fn key(diagnostic: &RangeExpected) -> (usize, usize, String, usize) {
    (
        diagnostic.line,
        diagnostic.column,
        diagnostic.message.clone(),
        diagnostic.length,
    )
}

fn inspect(registry: &CopRegistry, case: &RangeCase) -> Vec<RangeExpected> {
    let source = SourceFile::from_bytes(case_path(case), case.source.as_bytes().to_vec());
    let config = configuration(case);
    crate::linter::lint_source(
        &source,
        &config,
        registry,
        &CopFilterSet::build(&config, registry),
        Some(std::slice::from_ref(&case.cop)),
        &[],
        AutocorrectMode::Off,
        false,
    )
    .unwrap()
    .diagnostics
    .iter()
    .map(range_expected)
    .collect()
}

fn case_path(case: &RangeCase) -> String {
    case.path.replace(
        "$PROJECT",
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/ported_project"),
    )
}

fn configuration(case: &RangeCase) -> ResolvedConfig {
    let mut config = ResolvedConfig::empty();
    config.target_ruby_version = Some(3.4);
    config.display_cop_names = false;
    config.cop_configs.insert(
        case.cop.clone(),
        CopConfig {
            enabled: EnabledState::True,
            severity: case
                .config
                .get("Severity")
                .and_then(|value| value.as_str())
                .and_then(crate::diagnostic::Severity::from_str),
            include: vec!["**/*".into()],
            options: case.config.clone(),
            ..CopConfig::default()
        },
    );
    config
}

fn range_expected(diagnostic: &Diagnostic) -> RangeExpected {
    RangeExpected {
        line: diagnostic.location.line,
        column: diagnostic.location.column,
        message: diagnostic.message.clone(),
        length: diagnostic.highlight_length,
        severity: format!("{:?}", diagnostic.severity).to_lowercase(),
    }
}

mod annotations;
