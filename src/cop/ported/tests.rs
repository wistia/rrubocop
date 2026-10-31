mod fixtures;
mod runner;
use super::*;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Clone)]
struct Case {
    cop: String,
    source: String,
    path: String,
    config: HashMap<String, serde_yml::Value>,
    expected: Vec<Expected>,
    rust_expected: Option<Vec<Expected>>,
    corrected: String,
    rust_corrected: Option<String>,
    rust_correctable: Option<bool>,
    reason: Option<String>,
}
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
struct Expected {
    line: usize,
    column: usize,
    message: String,
    correctable: bool,
}

fn cases() -> Vec<Case> {
    serde_json::from_str(include_str!(
        "../../../tests/fixtures/ported-reference.json"
    ))
    .unwrap()
}

#[test]
fn pinned_ruby_diagnostics_and_corrections() {
    let registry = CopRegistry::default_registry();
    let failures = cases()
        .iter()
        .enumerate()
        .filter_map(|(index, case)| case_difference(&registry, index, case))
        .collect::<Vec<_>>();
    assert!(
        failures.is_empty(),
        "{} mismatches:\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}
fn case_difference(registry: &CopRegistry, index: usize, case: &Case) -> Option<String> {
    let (actual, corrected) = runner::run(registry, case);
    let expected = runner::expected(case);
    if case.rust_corrected.is_some() || case.rust_expected.is_some() {
        assert!(case.reason.is_some());
    }
    if actual == expected && corrected == *runner::expected_correction(case) {
        return None;
    }
    Some(format!(
        "{index} {} {:?}\nexpected={expected:?}\nactual={actual:?}\ncorrection expected={:?}\ncorrection actual={corrected:?}",
        case.cop, case.source, case.corrected
    ))
}

#[test]
fn all_ported_cops_have_fixture_pairs() {
    fixtures::check_pairs(&cases());
}
