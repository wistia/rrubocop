use crate::cli::AutocorrectMode;
use crate::config::{CopFilterSet, ResolvedConfig};
use crate::cop::{CopConfig, CopRegistry, EnabledState};
use std::path::Path;

#[test]
fn quote_removal_preserves_pinned_gaps_in_every_mode() {
    let cases: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/triple-quote-gap-safety.json"
    )))
    .unwrap();
    let registry = CopRegistry::default_registry();
    let config = configuration();
    for case in cases.as_array().unwrap() {
        for mode in [
            AutocorrectMode::Off,
            AutocorrectMode::Safe,
            AutocorrectMode::All,
        ] {
            verify(&registry, &config, case, mode);
        }
    }
}

fn configuration() -> ResolvedConfig {
    let mut config = ResolvedConfig::empty();
    config.target_ruby_version = Some(3.4);
    config.cop_configs.insert(
        "Lint/TripleQuotes".into(),
        CopConfig {
            enabled: EnabledState::True,
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
    let source = case["source"].as_str().unwrap();
    let mut bytes = source.as_bytes().to_vec();
    let diagnostics = crate::linter::lint_bytes_autocorrect(
        Path::new("probe.rb"),
        &mut bytes,
        config,
        registry,
        &CopFilterSet::build(config, registry),
        Some(&["Lint/TripleQuotes".into()]),
        &[],
        mode,
        false,
    )
    .unwrap();
    let key = if mode == AutocorrectMode::Off {
        "source"
    } else {
        "corrected"
    };
    assert_eq!(
        bytes,
        case[key].as_str().unwrap().as_bytes(),
        "{mode:?} {source}"
    );
    assert_eq!(
        diagnostics.len(),
        case["expected"].as_array().unwrap().len(),
        "{source}"
    );
    assert!(diagnostics.iter().all(|d| d.correctable));
}

#[test]
fn annotated_fixture_records_the_continuation_hazard() {
    let fixture = crate::testutil::parse_fixture(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/cops/lint/triple_quotes/continuation_autocorrect.rb"
    )));
    assert_eq!(fixture.source, b"p \"\"\"a\" \\\n\"#{__LINE__}\"\"\"\n");
    assert_eq!(fixture.expected.len(), 1);
}
