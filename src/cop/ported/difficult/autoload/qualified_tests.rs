use crate::cli::AutocorrectMode;
use crate::config::{CopFilterSet, ResolvedConfig};
use crate::cop::{CopConfig, CopRegistry, EnabledState};
use crate::parse::source::SourceFile;

const CASES: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/autoload-qualified-names.json"
));
const PROJECT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/ported_project");

#[test]
fn qualified_names_match_pinned_ruby_diagnostics_in_every_mode() {
    let cases: serde_json::Value = serde_json::from_str(CASES).unwrap();
    let registry = CopRegistry::default_registry();
    for case in cases.as_array().unwrap() {
        for mode in [
            AutocorrectMode::Off,
            AutocorrectMode::Safe,
            AutocorrectMode::All,
        ] {
            compare_case(&registry, case, mode);
        }
    }
}

fn compare_case(registry: &CopRegistry, case: &serde_json::Value, mode: AutocorrectMode) {
    let source = SourceFile::from_bytes(
        case["path"].as_str().unwrap().replace("$PROJECT", PROJECT),
        case["source"].as_str().unwrap().as_bytes().to_vec(),
    );
    let config = configuration();
    let findings = crate::linter::lint_source(
        &source,
        &config,
        registry,
        &CopFilterSet::build(&config, registry),
        Some(&["Cops/NoUndiscoverableAutoLoadedConstants".into()]),
        &[],
        mode,
        false,
    )
    .unwrap();
    assert_eq!(
        diagnostic_values(&findings.diagnostics),
        *case["expected"].as_array().unwrap(),
        "{mode:?} {}",
        case["source"]
    );
    assert!(findings.corrections.is_none_or(|edits| edits.is_empty()));
}

fn configuration() -> ResolvedConfig {
    let mut config = ResolvedConfig::empty();
    config.target_ruby_version = Some(3.4);
    config.cop_configs.insert(
        "Cops/NoUndiscoverableAutoLoadedConstants".into(),
        CopConfig {
            enabled: EnabledState::True,
            include: vec!["**/*".into()],
            ..CopConfig::default()
        },
    );
    config
}

#[test]
fn annotated_fixtures_cover_multiline_names_at_the_correct_autoload_path() {
    let cases: serde_json::Value = serde_json::from_str(CASES).unwrap();
    for (index, bytes) in [
        (1, include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/cops/cops/no_undiscoverable_auto_loaded_constants/qualified_no_offense.rb")).as_slice()),
        (12, include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/cops/cops/no_undiscoverable_auto_loaded_constants/qualified_offense.rb")).as_slice()),
    ] {
        let parsed = crate::testutil::parse_fixture(bytes);
        assert_eq!(parsed.source, cases[index]["source"].as_str().unwrap().as_bytes());
        assert_eq!(parsed.expected.len(), cases[index]["expected"].as_array().unwrap().len());
    }
}

fn diagnostic_values(diagnostics: &[crate::diagnostic::Diagnostic]) -> Vec<serde_json::Value> {
    diagnostics.iter().map(|d| {
        assert!(!d.correctable && !d.corrected);
        serde_json::json!({"line": d.location.line, "column": d.location.column,
            "message": d.message.trim_start_matches("Cops/NoUndiscoverableAutoLoadedConstants: "), "correctable": false})
    }).collect()
}
