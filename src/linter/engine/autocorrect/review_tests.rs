use super::*;
use crate::cop::{CopConfig, EnabledState};
use serde_json::Value;

#[test]
fn review_regressions_match_bundled_ruby_diagnostics_and_corrected_bytes() {
    let reference: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/engine/review-fixes-reference.json"
    )))
    .unwrap();
    for case in reference["cases"].as_array().unwrap() {
        check(case);
    }
}
fn check(case: &Value) {
    let only = selected(case);
    let config = configuration(&only);
    let registry = CopRegistry::default_registry();
    let filters = CopFilterSet::build(&config, &registry);
    let mode = mode(case);
    let mut bytes = case["source"].as_str().unwrap().as_bytes().to_vec();
    let diagnostics = lint_bytes_autocorrect(
        Path::new("probe.rb"),
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
    assert_bytes(case, &bytes);
    assert_offenses(case, &diagnostics);
}
fn assert_offenses(case: &Value, diagnostics: &[Diagnostic]) {
    let mut actual: Vec<_> = diagnostics.iter().map(|d| serde_json::json!({"cop": d.cop_name, "line":d.location.line, "column":d.location.column, "length": d.highlight_length, "message":d.message, "corrected":d.corrected, "correctable":d.correctable}).to_string()).collect();
    let mut expected: Vec<_> = case["offenses"].as_array().unwrap().iter().map(|o| serde_json::json!({"cop":o["cop_name"], "line":o["location"]["start_line"], "column":o["location"]["start_column"].as_u64().unwrap()-1, "length":o["location"]["length"], "message":o["message"], "corrected":o["corrected"], "correctable":o["correctable"]}).to_string()).collect();
    actual.sort();
    expected.sort();
    assert_eq!(actual, expected, "{} {}", case["name"], case["mode"]);
}

fn selected(case: &Value) -> Vec<String> {
    case["cops"]
        .as_array()
        .unwrap()
        .iter()
        .map(|cop| cop.as_str().unwrap().to_string())
        .collect()
}
fn configuration(only: &[String]) -> ResolvedConfig {
    let mut config = ResolvedConfig::empty();
    config.target_ruby_version = Some(3.4);
    config.display_cop_names = false;
    for name in only {
        config.cop_configs.insert(
            name.clone(),
            CopConfig {
                enabled: EnabledState::True,
                ..CopConfig::default()
            },
        );
    }
    config
}

fn mode(case: &Value) -> AutocorrectMode {
    match case["mode"].as_str().unwrap() {
        "-a" => AutocorrectMode::Safe,
        "-A" => AutocorrectMode::All,
        _ => AutocorrectMode::Off,
    }
}

fn assert_bytes(case: &Value, bytes: &[u8]) {
    if case.get("rust_corrected").is_some() {
        assert!(case.get("reason").is_some());
    }
    let expected = case.get("rust_corrected").unwrap_or(&case["corrected"]);
    assert_eq!(
        bytes,
        expected.as_str().unwrap().as_bytes(),
        "{} {}",
        case["name"],
        case["mode"]
    );
}
