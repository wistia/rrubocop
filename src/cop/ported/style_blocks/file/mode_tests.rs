use crate::cli::AutocorrectMode;
use crate::config::{CopFilterSet, ResolvedConfig};
use crate::cop::{CopConfig, CopRegistry, EnabledState};
use std::path::Path;

fn cases() -> serde_json::Value {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/file-mode-literal-safety.json"
    )))
    .unwrap()
}

#[test]
fn file_modes_match_pinned_references_in_all_modes() {
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
        expected(case, mode).as_bytes(),
        "{name} {mode:?} {case}"
    );
    assert_eq!(
        diagnostics.len(),
        case["expected"].as_array().unwrap().len()
    );
    assert!(diagnostics.iter().all(|d| d.correctable));
}

fn expected(case: &serde_json::Value, mode: AutocorrectMode) -> &str {
    let key = if mode == AutocorrectMode::Off {
        "source"
    } else {
        "corrected"
    };
    case[key].as_str().unwrap()
}

#[test]
fn annotated_fixtures_cover_mode_values_and_negative_controls() {
    let cases = cases();
    for (index, bytes) in [
        (
            0,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/file_read/quoted_mode_safety.rb"
            ))
            .as_slice(),
        ),
        (
            3,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/file_read/percent_mode_safety.rb"
            ))
            .as_slice(),
        ),
        (
            20,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/file_write/quoted_mode_safety.rb"
            ))
            .as_slice(),
        ),
        (
            27,
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/file_write/character_mode_safety.rb"
            ))
            .as_slice(),
        ),
    ] {
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

#[test]
fn heredoc_modes_cannot_leave_orphaned_bodies() {
    let registry = CopRegistry::default_registry();
    for (name, operation, mode_value) in [
        ("Style/FileRead", "read", "r"),
        ("Style/FileWrite", "write(\"new\")", "w"),
    ] {
        let source = format!(
            "File.open(\"sample.txt\", <<~MODE) {{ |f| f.{operation} }}\n{mode_value}\\\nMODE\n"
        );
        let case = serde_json::json!({
            "cop": name,
            "source": source,
            "corrected": source,
            "config": {"TargetRubyVersion": 3.4},
            "expected": []
        });
        let config = configuration(&case);
        for mode in [
            AutocorrectMode::Off,
            AutocorrectMode::Safe,
            AutocorrectMode::All,
        ] {
            verify(&registry, &config, &case, mode);
        }
    }
}
