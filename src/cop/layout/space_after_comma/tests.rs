use super::*;
use crate::cli::AutocorrectMode;
use crate::config::{CopFilterSet, ResolvedConfig};
use crate::cop::{CopRegistry, EnabledState};
use std::path::Path;

crate::cop_fixture_tests!(SpaceAfterComma, "cops/layout/space_after_comma");

#[test]
fn interpolated_code_is_corrected_without_changing_literal_commas() {
    let registry = CopRegistry::default_registry();
    let config = configuration();
    let filters = CopFilterSet::build(&config, &registry);
    let reference: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/cops/layout/space_after_comma/interpolation-reference.json"
    )))
    .unwrap();
    for case in reference["cases"].as_array().unwrap() {
        for mode in [
            AutocorrectMode::Off,
            AutocorrectMode::Safe,
            AutocorrectMode::All,
        ] {
            check_correction(case, mode, &registry, &config, &filters);
        }
    }
}
fn check_correction(
    case: &serde_json::Value,
    mode: AutocorrectMode,
    registry: &CopRegistry,
    config: &ResolvedConfig,
    filters: &CopFilterSet,
) {
    let text = case["source"].as_str().unwrap();
    let mut bytes = text.as_bytes().to_vec();
    let diagnostics = crate::linter::lint_bytes_autocorrect(
        Path::new("t.rb"),
        &mut bytes,
        config,
        registry,
        filters,
        Some(&["Layout/SpaceAfterComma".into()]),
        &[],
        mode,
        false,
    )
    .unwrap();
    assert_offenses(&diagnostics, case);
    let expected = if mode == AutocorrectMode::Off {
        text
    } else {
        case["corrected"].as_str().unwrap()
    };
    assert_eq!(String::from_utf8(bytes).unwrap(), expected, "{text}");
}
fn assert_offenses(diagnostics: &[Diagnostic], case: &serde_json::Value) {
    let actual: Vec<_> = diagnostics
        .iter()
        .map(|d| {
            serde_json::json!({
                "line": d.location.line, "column": d.location.column,
                "length": d.highlight_length, "message": d.message,
                "correctable": d.correctable,
            })
        })
        .collect();
    assert_eq!(
        serde_json::json!(actual),
        case["offenses"],
        "{}",
        case["source"]
    );
}

fn configuration() -> ResolvedConfig {
    let mut config = ResolvedConfig::empty();
    config.target_ruby_version = Some(3.4);
    config.display_cop_names = false;
    config.cop_configs.insert(
        "Layout/SpaceAfterComma".into(),
        CopConfig {
            enabled: EnabledState::True,
            ..CopConfig::default()
        },
    );
    config
}
