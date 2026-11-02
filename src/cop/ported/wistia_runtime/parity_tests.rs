use crate::cli::AutocorrectMode;
use crate::config::{CopFilterSet, ResolvedConfig};
use crate::cop::{CopConfig, CopRegistry, EnabledState};
use serde_json::Value;
use std::path::Path;

fn cases() -> Vec<Value> {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/custom-call-parity-regressions.json"
    )))
    .unwrap()
}

#[test]
fn comment_arguments_and_encoded_coder_keys_match_ruby_in_every_mode() {
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

fn inspect(registry: &CopRegistry, case: &Value, mode: AutocorrectMode) {
    let source = case["source"].as_str().unwrap();
    let mut bytes = source.as_bytes().to_vec();
    let actual = run(registry, case, mode, &mut bytes)
        .into_iter()
        .map(diagnostic)
        .collect::<Vec<_>>();
    assert_eq!(actual, expected(case), "{} {mode:?}: {source}", case["cop"]);
    assert_eq!(bytes, source.as_bytes());
}

fn run(
    registry: &CopRegistry,
    case: &Value,
    mode: AutocorrectMode,
    bytes: &mut Vec<u8>,
) -> Vec<crate::diagnostic::Diagnostic> {
    let config = configuration(case);
    crate::linter::lint_bytes_autocorrect(
        Path::new(&case["path"].as_str().unwrap().replace(
            "$PROJECT",
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/ported_project"),
        )),
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

fn diagnostic(diagnostic: crate::diagnostic::Diagnostic) -> String {
    assert!(!diagnostic.corrected);
    format!(
        "{}:{}:{}:{}",
        diagnostic.location.line,
        diagnostic.location.column,
        diagnostic.correctable,
        diagnostic.message
    )
}

fn expected(case: &Value) -> Vec<String> {
    case["expected"]
        .as_array()
        .unwrap()
        .iter()
        .map(|offense| {
            format!(
                "{}:{}:{}:{}",
                offense["line"],
                offense["column"],
                offense["correctable"],
                offense["message"].as_str().unwrap()
            )
        })
        .collect()
}

#[test]
fn annotated_custom_call_regressions_match_reference_inputs() {
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
