use crate::cli::AutocorrectMode;
use crate::config::{CopFilterSet, ResolvedConfig};
use crate::cop::{CopConfig, CopRegistry, EnabledState};
use serde_json::Value;
use std::path::Path;

#[test]
fn four_retained_rules_preserve_unsafe_shapes_and_correct_literal_controls() {
    let cases: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/four-retained-safety.json"
    )))
    .unwrap();
    for case in cases.as_array().unwrap() {
        for mode in [
            AutocorrectMode::Off,
            AutocorrectMode::Safe,
            AutocorrectMode::All,
        ] {
            check(case, mode);
        }
    }
}
fn check(case: &Value, mode: AutocorrectMode) {
    let (bytes, diagnostics) = run(case, mode);
    let source = case["source"].as_str().unwrap();
    let expected = if mode == AutocorrectMode::Off || case["manual"] == true {
        source
    } else {
        case["corrected"].as_str().unwrap()
    };
    assert_eq!(bytes, expected.as_bytes(), "{}: {source}", case["cop"]);
    if case["manual"] == true {
        assert!(diagnostics.iter().all(|d| !d.correctable && !d.corrected));
    }
    assert_metadata(case, &diagnostics);
}
fn run(case: &Value, mode: AutocorrectMode) -> (Vec<u8>, Vec<crate::diagnostic::Diagnostic>) {
    let name = case["cop"].as_str().unwrap();
    let config = configuration(name);
    let registry = CopRegistry::default_registry();
    let mut bytes = case["source"].as_str().unwrap().as_bytes().to_vec();
    let diagnostics = crate::linter::lint_bytes_autocorrect(
        Path::new("probe.rb"),
        &mut bytes,
        &config,
        &registry,
        &CopFilterSet::build(&config, &registry),
        Some(&[name.into()]),
        &[],
        mode,
        false,
    )
    .unwrap();
    (bytes, diagnostics)
}
fn assert_metadata(case: &Value, diagnostics: &[crate::diagnostic::Diagnostic]) {
    let actual: Vec<_> = diagnostics
        .iter()
        .map(|d| (d.location.line, d.location.column, d.message.as_str()))
        .collect();
    let expected: Vec<_> = case["expected"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| {
            (
                d["line"].as_u64().unwrap() as usize,
                d["column"].as_u64().unwrap() as usize,
                d["message"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(actual, expected, "{} {}", case["cop"], case["source"]);
}

fn configuration(name: &str) -> ResolvedConfig {
    let mut config = ResolvedConfig::empty();
    config.target_ruby_version = Some(3.4);
    config.display_cop_names = false;
    config.cop_configs.insert(
        name.into(),
        CopConfig {
            enabled: EnabledState::True,
            ..CopConfig::default()
        },
    );
    config
}

#[test]
fn unsafe_correction_fixtures_keep_their_diagnostics() {
    let registry = CopRegistry::default_registry();
    for (name, fixture) in [
        ("Style/KeywordParametersOrder", include_bytes!("../../../../../tests/fixtures/cops/style/keyword_parameters_order/unsafe_autocorrect.rb").as_slice()),
        ("Style/TrailingBodyOnMethodDefinition", include_bytes!("../../../../../tests/fixtures/cops/style/trailing_body_on_method_definition/unsafe_autocorrect.rb").as_slice()),
        ("Style/FileWrite", include_bytes!("../../../../../tests/fixtures/cops/style/file_write/unsafe_autocorrect.rb").as_slice()),
        ("Style/RedundantAssignment", include_bytes!("../../../../../tests/fixtures/cops/style/redundant_assignment/unsafe_autocorrect.rb").as_slice()),
    ] { crate::testutil::assert_cop_offenses_full(registry.get(name).unwrap(), fixture); }
}
