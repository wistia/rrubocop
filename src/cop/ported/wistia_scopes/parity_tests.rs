use crate::cli::AutocorrectMode;
use crate::config::{CopFilterSet, ResolvedConfig};
use crate::cop::{CopConfig, CopRegistry, EnabledState};
use crate::parse::source::SourceFile;

const CASES: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/wistia-scope-parity.json"
));

#[test]
fn scope_forms_match_pinned_ruby_diagnostics_in_every_mode() {
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
    let name = case["cop"].as_str().unwrap();
    let config = configuration(name);
    let findings = crate::linter::lint_source(
        &case_source(case),
        &config,
        registry,
        &CopFilterSet::build(&config, registry),
        Some(&[name.into()]),
        &[],
        mode,
        false,
    )
    .unwrap();
    assert_eq!(
        diagnostic_values(&findings.diagnostics, name),
        *case["expected"].as_array().unwrap(),
        "{mode:?} {}",
        case["source"]
    );
    assert!(findings.corrections.is_none_or(|edits| edits.is_empty()));
}

fn configuration(name: &str) -> ResolvedConfig {
    let mut config = ResolvedConfig::empty();
    config.target_ruby_version = Some(3.4);
    config.cop_configs.insert(
        name.into(),
        CopConfig {
            enabled: EnabledState::True,
            include: vec!["**/*".into()],
            ..CopConfig::default()
        },
    );
    config
}

#[test]
fn annotated_fixtures_cover_each_scope_rule() {
    let cases: serde_json::Value = serde_json::from_str(CASES).unwrap();
    let fixtures: [(usize, &[u8]); 6] = [
        (
            0,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/cops/no_constant_definitions_in_specs/scope_offense.rb"
            )),
        ),
        (
            11,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/cops/no_constant_definitions_in_specs/scope_no_offense.rb"
            )),
        ),
        (
            18,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/rails/reversible_migration_method_definition/scope_offense.rb"
            )),
        ),
        (
            16,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/rails/reversible_migration_method_definition/scope_no_offense.rb"
            )),
        ),
        (
            33,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/cops/no_top_level_side_effects_in_autoloaded_files/scope_offense.rb"
            )),
        ),
        (
            31,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/cops/no_top_level_side_effects_in_autoloaded_files/scope_no_offense.rb"
            )),
        ),
    ];
    for (index, bytes) in fixtures {
        let parsed = crate::testutil::parse_fixture(bytes);
        assert_eq!(
            parsed.source,
            cases[index]["source"].as_str().unwrap().as_bytes()
        );
        assert_eq!(
            parsed.expected.len(),
            cases[index]["expected"].as_array().unwrap().len()
        );
    }
}

fn diagnostic_values(
    diagnostics: &[crate::diagnostic::Diagnostic],
    name: &str,
) -> Vec<serde_json::Value> {
    diagnostics
        .iter()
        .map(|d| {
            assert!(!d.correctable && !d.corrected);
            serde_json::json!({"line": d.location.line, "column": d.location.column,
            "message": d.message.trim_start_matches(&format!("{name}: ")), "correctable": false})
        })
        .collect()
}

fn case_source(case: &serde_json::Value) -> SourceFile {
    SourceFile::from_bytes(
        case["path"].as_str().unwrap(),
        case["source"].as_str().unwrap().as_bytes().to_vec(),
    )
}
