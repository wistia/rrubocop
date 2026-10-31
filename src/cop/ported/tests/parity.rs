use super::*;
use crate::cli::AutocorrectMode;
use crate::config::{CopFilterSet, ResolvedConfig};
use crate::cop::EnabledState;

#[test]
fn independent_review_regressions_match_pinned_ruby_in_the_real_pipeline() {
    let registry = CopRegistry::default_registry();
    let cases: Vec<Case> = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/final-parity-regressions.json"
    ))
    .unwrap();
    for case in cases {
        for mode in [
            AutocorrectMode::Off,
            AutocorrectMode::Safe,
            AutocorrectMode::All,
        ] {
            inspect(&registry, &case, mode);
        }
    }
}
fn configuration(case: &Case) -> ResolvedConfig {
    let mut config = ResolvedConfig::empty();
    config.display_cop_names = false;
    config.cop_configs.insert(
        case.cop.clone(),
        CopConfig {
            enabled: EnabledState::True,
            options: case.config.clone(),
            ..CopConfig::default()
        },
    );
    config
}
fn inspect(registry: &CopRegistry, case: &Case, mode: AutocorrectMode) {
    let config = configuration(case);
    let mut bytes = case.source.as_bytes().to_vec();
    let actual = crate::linter::lint_bytes_autocorrect(
        std::path::Path::new(&case.path),
        &mut bytes,
        &config,
        registry,
        &CopFilterSet::build(&config, registry),
        Some(std::slice::from_ref(&case.cop)),
        &[],
        mode,
        false,
    )
    .unwrap();
    assert_eq!(
        diagnostics(actual),
        runner::expected(case),
        "{} {mode:?} {:?}",
        case.cop,
        case.source
    );
    let expected = if mode == AutocorrectMode::Off {
        &case.source
    } else {
        runner::expected_correction(case)
    };
    assert_eq!(
        String::from_utf8(bytes).unwrap(),
        *expected,
        "{} {mode:?} {:?}",
        case.cop,
        case.source
    );
}

fn diagnostics(values: Vec<Diagnostic>) -> Vec<String> {
    let mut actual = values
        .into_iter()
        .map(|d| {
            format!(
                "{}:{}:{}:{}",
                d.location.line, d.location.column, d.correctable, d.message
            )
        })
        .collect::<Vec<_>>();
    actual.sort();
    actual
}
