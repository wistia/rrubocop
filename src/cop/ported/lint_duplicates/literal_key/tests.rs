use super::*;
use crate::cli::AutocorrectMode;
use crate::config::{CopFilterSet, ResolvedConfig};
use crate::cop::{CopConfig, CopRegistry, EnabledState};
use std::path::Path;

fn cases() -> serde_json::Value {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/semantic-literal-key-parity.json"
    )))
    .unwrap()
}

#[test]
fn semantic_keys_match_pinned_nodes_and_printed_require_values() {
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
    let fixtures: [(&str, &[u8]); 4] = [
        (
            "Lint/DuplicateCaseCondition",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/lint/duplicate_case_condition/semantic_numeric_keys.rb"
            )),
        ),
        (
            "Lint/DuplicateCaseCondition",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/lint/duplicate_case_condition/binary_encoding_no_offense.rb"
            )),
        ),
        (
            "Lint/DuplicateElsifCondition",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/lint/duplicate_elsif_condition/semantic_literal_keys.rb"
            )),
        ),
        (
            "Lint/DuplicateRequire",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/lint/duplicate_require/printed_zero_no_offense.rb"
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

#[test]
fn nested_keys_grow_linearly_instead_of_escaping_previous_keys() {
    let source = format!("{}\"a\"{}", "[".repeat(12), "]".repeat(12));
    let file = crate::parse::source::SourceFile::from_bytes("test.rb", source.as_bytes().to_vec());
    let tree = crate::parse::parse_ruby(&file).unwrap();
    let config = CopConfig::default();
    let context = Context::new(&file, &config);
    let key = literal_key(&context, tree.root_node().named_child(0).unwrap());
    assert!(key.len() < source.len() * 16, "{}", key.len());
}
