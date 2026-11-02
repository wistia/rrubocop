use crate::cli::AutocorrectMode;
use crate::config::{CopFilterSet, ResolvedConfig};
use crate::cop::{CopConfig, CopRegistry, EnabledState};
use crate::diagnostic::Diagnostic;
use serde_json::Value;
use std::path::Path;

fn cases() -> Vec<Value> {
    let mut cases: Vec<Value> = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/comment-argument-parity.json"
    )))
    .unwrap();
    cases.extend(
        serde_json::from_str::<Vec<Value>>(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/semantic-argument-parity.json"
        )))
        .unwrap(),
    );
    cases.extend(
        serde_json::from_str::<Vec<Value>>(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/block-receiver-parity.json"
        )))
        .unwrap(),
    );
    cases
}

#[test]
fn argument_comments_match_pinned_ruby_in_every_correction_mode() {
    let registry = CopRegistry::default_registry();
    for case in cases() {
        for mode in [
            AutocorrectMode::Off,
            AutocorrectMode::Safe,
            AutocorrectMode::All,
        ] {
            inspect(&registry, &case, mode);
        }
    }
}

fn configuration(case: &Value) -> ResolvedConfig {
    let mut config = ResolvedConfig::empty();
    config.display_cop_names = false;
    config.target_ruby_version = Some(3.4);
    config.cop_configs.insert(
        case["cop"].as_str().unwrap().into(),
        CopConfig {
            enabled: EnabledState::True,
            include: vec!["**/*".into()],
            options: serde_json::from_value(case["config"].clone()).unwrap(),
            ..CopConfig::default()
        },
    );
    config
}

fn run(
    registry: &CopRegistry,
    case: &Value,
    mode: AutocorrectMode,
    bytes: &mut Vec<u8>,
) -> Vec<Diagnostic> {
    let config = configuration(case);
    crate::linter::lint_bytes_autocorrect(
        Path::new(case["path"].as_str().unwrap()),
        bytes,
        &config,
        registry,
        &CopFilterSet::build(&config, registry),
        Some(&[case["cop"].as_str().unwrap().into()]),
        &[],
        mode,
        false,
    )
    .unwrap()
}

fn inspect(registry: &CopRegistry, case: &Value, mode: AutocorrectMode) {
    let mut bytes = case["source"].as_str().unwrap().as_bytes().to_vec();
    let mut actual = run(registry, case, mode, &mut bytes)
        .into_iter()
        .map(diagnostic)
        .collect::<Vec<_>>();
    actual.sort();
    assert_eq!(
        actual,
        expected(case),
        "{} {mode:?}: {}",
        case["cop"],
        case["source"]
    );
    assert_eq!(
        String::from_utf8(bytes).unwrap(),
        expected_source(registry, case, mode)
    );
}

fn diagnostic(diagnostic: Diagnostic) -> String {
    format!(
        "{}:{}:{}:{}",
        diagnostic.location.line,
        diagnostic.location.column,
        diagnostic.correctable,
        diagnostic.message
    )
}

fn expected(case: &Value) -> Vec<String> {
    let mut expected = case
        .get("rust_expected")
        .unwrap_or(&case["expected"])
        .as_array()
        .unwrap()
        .iter()
        .map(|offense| expected_diagnostic(case, offense))
        .collect::<Vec<_>>();
    expected.sort();
    expected
}
fn expected_diagnostic(case: &Value, offense: &Value) -> String {
    format!(
        "{}:{}:{}:{}",
        offense["line"],
        offense["column"],
        case.get("rust_correctable")
            .unwrap_or(&offense["correctable"]),
        offense["message"].as_str().unwrap()
    )
}
fn correction_allowed(registry: &CopRegistry, case: &Value, mode: AutocorrectMode) -> bool {
    if mode == AutocorrectMode::Off {
        return false;
    }
    let name = case["cop"].as_str().unwrap();
    let cop = &configuration(case).cop_configs[name];
    let safe = cop.get_bool("Safe", true)
        && cop.get_bool(
            "SafeAutoCorrect",
            registry.get(name).unwrap().safe_autocorrect(),
        );
    mode == AutocorrectMode::All || safe
}
fn expected_source<'a>(registry: &CopRegistry, case: &'a Value, mode: AutocorrectMode) -> &'a str {
    if !correction_allowed(registry, case, mode) {
        return case["source"].as_str().unwrap();
    }
    case.get("rust_corrected")
        .unwrap_or(&case["corrected"])
        .as_str()
        .unwrap()
}

#[test]
fn annotated_argument_comment_fixtures_match_reference_inputs() {
    for case in cases() {
        let Some(fixture) = case["fixture"].as_str() else {
            continue;
        };
        let parsed = crate::testutil::parse_fixture(
            &std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join(fixture)).unwrap(),
        );
        assert_eq!(parsed.source, case["source"].as_str().unwrap().as_bytes());
        assert_eq!(
            parsed.expected.len(),
            case["expected"].as_array().unwrap().len()
        );
    }
}
