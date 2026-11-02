use crate::cli::AutocorrectMode;
use crate::config::{CopFilterSet, ResolvedConfig};
use crate::cop::{CopConfig, CopRegistry, EnabledState};
use crate::diagnostic::Diagnostic;
use std::path::Path;

const CASES: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/scope-shape-parity.json"
));
const PROJECT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/ported_project");

#[test]
fn scope_shapes_match_pinned_diagnostics_without_corrections() {
    check_cases(CASES);
}

#[test]
fn disabled_shape_findings_remain_suppressed_in_every_mode() {
    check_cases(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/scope-shape-directives.json"
    )));
}

fn check_cases(json: &str) {
    let cases: serde_json::Value = serde_json::from_str(json).unwrap();
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
    let (after, diagnostics) = run(registry, case, mode);
    assert_eq!(after, case["source"].as_str().unwrap());
    assert_eq!(
        diagnostic_values(&diagnostics),
        expected(case),
        "{mode:?}: {}",
        case["source"]
    );
}

fn run(
    registry: &CopRegistry,
    case: &serde_json::Value,
    mode: AutocorrectMode,
) -> (String, Vec<Diagnostic>) {
    let name = case["cop"].as_str().unwrap();
    let config = configuration(case);
    let mut source = case["source"].as_str().unwrap().as_bytes().to_vec();
    let diagnostics = crate::linter::lint_bytes_autocorrect(
        Path::new(&case_path(case)),
        &mut source,
        &config,
        registry,
        &CopFilterSet::build(&config, registry),
        Some(&[name.into()]),
        &[],
        mode,
        false,
    )
    .unwrap();
    (String::from_utf8(source).unwrap(), diagnostics)
}

fn case_path(case: &serde_json::Value) -> String {
    case["path"].as_str().unwrap().replace("$PROJECT", PROJECT)
}

fn configuration(case: &serde_json::Value) -> ResolvedConfig {
    let mut config = ResolvedConfig::empty();
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

fn expected(case: &serde_json::Value) -> Vec<serde_json::Value> {
    case.get("rust_expected")
        .unwrap_or(&case["expected"])
        .as_array()
        .unwrap()
        .iter()
        .cloned()
        .map(|mut offense| {
            if case["rust_correctable"] == false {
                offense["correctable"] = false.into();
            }
            offense
        })
        .collect()
}

fn diagnostic_values(diagnostics: &[Diagnostic]) -> Vec<serde_json::Value> {
    diagnostics.iter().map(|d| {
        assert!(!d.correctable && !d.corrected);
        let message = d.message.split_once(": ").map_or(d.message.as_str(), |(_, message)| message);
        serde_json::json!({"line": d.location.line, "column": d.location.column, "message": message, "correctable": false})
    }).collect()
}

#[test]
fn annotated_fixtures_cover_each_shape_regression() {
    let cases: serde_json::Value = serde_json::from_str(CASES).unwrap();
    let fixtures: [(usize, &[u8]); 8] = [
        (
            0,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/cops/no_undiscoverable_auto_loaded_constants/shape_offense.rb"
            )),
        ),
        (
            3,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/cops/no_undiscoverable_auto_loaded_constants/shape_no_offense.rb"
            )),
        ),
        (
            9,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/cops/no_top_level_side_effects_in_autoloaded_files/shape_offense.rb"
            )),
        ),
        (
            13,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/cops/no_top_level_side_effects_in_autoloaded_files/shape_no_offense.rb"
            )),
        ),
        (
            24,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/mixin_grouping/shape_offense.rb"
            )),
        ),
        (
            29,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/mixin_grouping/shape_no_offense.rb"
            )),
        ),
        (
            36,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/cops/no_constant_definitions_in_specs/shape_offense.rb"
            )),
        ),
        (
            50,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/cops/no_constant_definitions_in_specs/shape_no_offense.rb"
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
