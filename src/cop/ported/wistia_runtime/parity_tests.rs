use crate::cli::AutocorrectMode;
use crate::config::{CopFilterSet, ResolvedConfig};
use crate::cop::{CopConfig, CopRegistry, EnabledState};
use serde_json::Value;
use std::path::Path;

fn cases() -> Vec<Value> {
    let mut cases: Vec<Value> = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/custom-call-parity-regressions.json"
    )))
    .unwrap();
    cases.extend(
        serde_json::from_str::<Vec<Value>>(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/custom-runtime-shape-parity.json"
        )))
        .unwrap(),
    );
    cases
}

#[test]
fn custom_runtime_shapes_match_ruby_in_every_mode() {
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

#[test]
fn rescue_modifier_directives_follow_the_keyword_not_string_contents() {
    let registry = CopRegistry::default_registry();
    let reference = cases()
        .into_iter()
        .find(|case| case["source"] == "work(\n \"rescue\"\n) rescue nil\n")
        .unwrap();
    for mode in [
        AutocorrectMode::Off,
        AutocorrectMode::Safe,
        AutocorrectMode::All,
    ] {
        let mut keyword_disabled = reference.clone();
        keyword_disabled["source"] =
            "work(\n \"rescue\"\n) rescue nil # rubocop:disable Cops/RescueMustReportError\n"
                .into();
        keyword_disabled["expected"] = serde_json::json!([]);
        inspect(&registry, &keyword_disabled, mode);
        let mut string_disabled = reference.clone();
        string_disabled["source"] =
            "work(\n \"rescue\" # rubocop:disable Cops/RescueMustReportError\n) rescue nil\n"
                .into();
        inspect(&registry, &string_disabled, mode);
    }
}
