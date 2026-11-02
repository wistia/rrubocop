use crate::cli::AutocorrectMode;
use crate::config::{CopFilterSet, ResolvedConfig};
use crate::cop::{CopConfig, CopRegistry, EnabledState};
use std::path::Path;

fn cases() -> serde_json::Value {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/literal-suffix-safety.json"
    )))
    .unwrap()
}

#[test]
fn literal_matching_and_suffix_edits_match_pinned_references_in_all_modes() {
    let registry = CopRegistry::default_registry();
    for case in cases().as_array().unwrap() {
        let config = configuration(case);
        for mode in [
            AutocorrectMode::Off,
            AutocorrectMode::Safe,
            AutocorrectMode::All,
        ] {
            verify(&registry, &config, case, mode);
        }
    }
}

fn configuration(case: &serde_json::Value) -> ResolvedConfig {
    let mut config = ResolvedConfig::empty();
    config.target_ruby_version = Some(3.4);
    config.cop_configs.insert(
        case["cop"].as_str().unwrap().into(),
        CopConfig {
            enabled: EnabledState::True,
            options: serde_json::from_value(case["config"].clone()).unwrap(),
            ..CopConfig::default()
        },
    );
    config
}

fn verify(
    registry: &CopRegistry,
    config: &ResolvedConfig,
    case: &serde_json::Value,
    mode: AutocorrectMode,
) {
    let name = case["cop"].as_str().unwrap();
    let mut bytes = case["source"].as_str().unwrap().as_bytes().to_vec();
    let diagnostics = crate::linter::lint_bytes_autocorrect(
        Path::new("probe.rb"),
        &mut bytes,
        config,
        registry,
        &CopFilterSet::build(config, registry),
        Some(&[name.into()]),
        &[],
        mode,
        false,
    )
    .unwrap();
    assert_eq!(
        bytes,
        expected(case, mode).as_bytes(),
        "{name} {mode:?} {case}"
    );
    assert_eq!(
        diagnostics.len(),
        case["expected"].as_array().unwrap().len()
    );
    if case["manual"] == true {
        assert!(diagnostics.iter().all(|d| !d.correctable && !d.corrected));
    }
}

fn expected(case: &serde_json::Value, mode: AutocorrectMode) -> &str {
    let safe = case["cop"] != "Lint/BooleanSymbol";
    let key = if mode == AutocorrectMode::Off || (mode == AutocorrectMode::Safe && !safe) {
        "source"
    } else if case.get("rust_corrected").is_some() {
        "rust_corrected"
    } else {
        "corrected"
    };
    case[key].as_str().unwrap()
}

#[test]
fn annotated_fixtures_cover_false_positive_and_percent_suffix() {
    let cases = cases();
    for (index, bytes) in [
        (0, include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/cops/lint/boolean_symbol/quoted_content_safety.rb")).as_slice()),
        (14, include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/cops/style/redundant_file_extension_in_require/percent_suffix_safety.rb")).as_slice()),
    ] {
        let parsed = crate::testutil::parse_fixture(bytes);
        assert_eq!(parsed.source, cases[index]["source"].as_str().unwrap().as_bytes());
        assert_eq!(parsed.expected.len(), cases[index]["expected"].as_array().unwrap().len());
    }
}
