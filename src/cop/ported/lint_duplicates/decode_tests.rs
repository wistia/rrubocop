use crate::cli::AutocorrectMode;
use crate::config::{CopFilterSet, ResolvedConfig};
use crate::cop::{CopConfig, CopRegistry, EnabledState};
use std::path::Path;

fn cases() -> serde_json::Value {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/literal-decode-audit-parity.json"
    )))
    .unwrap()
}

#[test]
fn decoded_literals_match_pinned_diagnostics_and_all_correction_modes() {
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
        expected(registry, case, mode).as_bytes(),
        "{name} {mode:?} {case}"
    );
    verify_diagnostics(&diagnostics, case);
    if case["manual"] == true {
        assert!(diagnostics.iter().all(|d| !d.correctable && !d.corrected));
    }
}

fn expected<'a>(
    registry: &CopRegistry,
    case: &'a serde_json::Value,
    mode: AutocorrectMode,
) -> &'a str {
    let key = if mode == AutocorrectMode::Off
        || (mode == AutocorrectMode::Safe && !safe_correction(registry, case))
    {
        "source"
    } else if case.get("rust_corrected").is_some() {
        "rust_corrected"
    } else {
        "corrected"
    };
    case[key].as_str().unwrap()
}

fn safe_correction(registry: &CopRegistry, case: &serde_json::Value) -> bool {
    let cop = registry.get(case["cop"].as_str().unwrap()).unwrap();
    let config = CopConfig {
        options: serde_json::from_value(case["config"].clone()).unwrap(),
        ..CopConfig::default()
    };
    config.get_bool("Safe", true) && config.get_bool("SafeAutoCorrect", cop.safe_autocorrect())
}

#[test]
fn annotated_fixtures_record_each_literal_rule() {
    let cases = cases();
    let fixtures: [(&str, &[u8]); 8] = [
        (
            "Lint/BooleanSymbol",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/lint/boolean_symbol/literal_decode_parity.rb"
            )),
        ),
        (
            "Lint/DuplicateCaseCondition",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/lint/duplicate_case_condition/literal_decode_parity.rb"
            )),
        ),
        (
            "Lint/DuplicateRequire",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/lint/duplicate_require/literal_decode_parity.rb"
            )),
        ),
        (
            "Lint/RescueType",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/lint/rescue_type/literal_decode_parity.rb"
            )),
        ),
        (
            "Lint/TripleQuotes",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/lint/triple_quotes/literal_decode_parity.rb"
            )),
        ),
        (
            "Style/ClassEqualityComparison",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/class_equality_comparison/literal_decode_parity.rb"
            )),
        ),
        (
            "Style/HashExcept",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/hash_except/literal_decode_parity.rb"
            )),
        ),
        (
            "Style/RedundantFileExtensionInRequire",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/redundant_file_extension_in_require/literal_decode_parity.rb"
            )),
        ),
    ];
    for (name, bytes) in fixtures {
        verify_fixture(&cases, name, bytes);
    }
}

fn verify_fixture(cases: &serde_json::Value, name: &str, bytes: &[u8]) {
    let parsed = crate::testutil::parse_fixture(bytes);
    let case = cases
        .as_array()
        .unwrap()
        .iter()
        .find(|case| {
            case["cop"] == name && case["source"].as_str().unwrap().as_bytes() == parsed.source
        })
        .unwrap();
    assert_eq!(
        parsed.expected.len(),
        case["expected"].as_array().unwrap().len()
    );
}

fn verify_diagnostics(diagnostics: &[crate::diagnostic::Diagnostic], case: &serde_json::Value) {
    let actual = diagnostics.iter().map(diagnostic).collect::<Vec<_>>();
    let mut expected = case["expected"].as_array().unwrap().clone();
    if let Some(correctable) = case.get("rust_correctable") {
        for diagnostic in &mut expected {
            diagnostic["correctable"] = correctable.clone();
        }
    }
    assert_eq!(actual, expected, "{case}");
}

fn diagnostic(d: &crate::diagnostic::Diagnostic) -> serde_json::Value {
    let prefix = format!("{}: ", d.cop_name);
    serde_json::json!({
        "line": d.location.line,
        "column": d.location.column,
        "message": d.message.strip_prefix(&prefix).unwrap_or(&d.message),
        "correctable": d.correctable,
        "length": d.highlight_length,
        "severity": format!("{:?}", d.severity).to_lowercase()
    })
}
