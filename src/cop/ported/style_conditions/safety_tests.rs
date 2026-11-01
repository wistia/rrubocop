use super::*;
use crate::cli::AutocorrectMode;
use crate::config::{CopFilterSet, ResolvedConfig};
use crate::cop::{CopConfig, EnabledState};
use std::path::Path;

#[test]
fn branch_and_block_rewrites_preserve_reference_results_in_every_mode() {
    let cases: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/branch-block-safety.json"
    )))
    .unwrap();
    let registry = CopRegistry::default_registry();
    for case in cases.as_array().unwrap() {
        verify_case(&registry, case);
    }
}

fn verify_case(registry: &CopRegistry, case: &serde_json::Value) {
    let name = case["cop"].as_str().unwrap();
    let config = configuration(case);
    let cop = &config.cop_configs[name];
    let safe = cop.get_bool("Safe", true)
        && cop.get_bool(
            "SafeAutoCorrect",
            registry.get(name).unwrap().safe_autocorrect(),
        );
    for mode in [
        AutocorrectMode::Off,
        AutocorrectMode::Safe,
        AutocorrectMode::All,
    ] {
        verify_mode(registry, &config, case, mode, safe);
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

fn verify_mode(
    registry: &CopRegistry,
    config: &ResolvedConfig,
    case: &serde_json::Value,
    mode: AutocorrectMode,
    safe: bool,
) {
    let name = case["cop"].as_str().unwrap();
    let source = case["source"].as_str().unwrap();
    let (after, diagnostics) = correct(registry, config, name, source, mode);
    assert_eq!(
        after,
        expected(case, mode, safe),
        "{name}: {mode:?} {source}"
    );
    if case["manual"] == true {
        assert!(
            diagnostics.iter().all(|d| !d.correctable && !d.corrected),
            "{name}: {source}"
        );
    }
}

fn expected(case: &serde_json::Value, mode: AutocorrectMode, safe: bool) -> &str {
    let key = if mode == AutocorrectMode::Off || (mode == AutocorrectMode::Safe && !safe) {
        "source"
    } else if case.get("rust_corrected").is_some() {
        "rust_corrected"
    } else {
        "corrected"
    };
    case[key].as_str().unwrap()
}

fn correct(
    registry: &CopRegistry,
    config: &ResolvedConfig,
    name: &str,
    source: &str,
    mode: AutocorrectMode,
) -> (String, Vec<crate::diagnostic::Diagnostic>) {
    let mut bytes = source.as_bytes().to_vec();
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
    (String::from_utf8(bytes).unwrap(), diagnostics)
}
