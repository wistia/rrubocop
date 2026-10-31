use super::*;
use crate::cli::AutocorrectMode;
use crate::config::{CopFilterSet, ResolvedConfig};
use crate::cop::EnabledState;

#[test]
fn final_retained_checks_run_through_the_real_syntax_and_scope_pipeline() {
    let registry = CopRegistry::default_registry();
    for case in cases().into_iter().filter(|c| final_check(&c.cop)) {
        assert_eq!(
            inspect(&registry, &case),
            runner::expected(&case),
            "{} {:?}",
            case.cop,
            case.source
        );
    }
}
fn final_check(name: &str) -> bool {
    [
        "Lint/MixedRegexpCaptureTypes",
        "Lint/OutOfRangeRegexpRef",
        "Lint/ShadowedArgument",
        "Style/EvalWithLocation",
        "Cops/NoUndiscoverableAutoLoadedConstants",
    ]
    .contains(&name)
}
fn inspect(registry: &CopRegistry, case: &Case) -> Vec<String> {
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
    let mut bytes = case.source.as_bytes().to_vec();
    let path = std::path::PathBuf::from(case.path.replace(
        "$PROJECT",
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/ported_project"),
    ));
    let findings = crate::linter::lint_bytes_autocorrect(
        &path,
        &mut bytes,
        &config,
        registry,
        &CopFilterSet::build(&config, registry),
        Some(&[case.cop.clone()]),
        &[],
        AutocorrectMode::Off,
        false,
    )
    .unwrap();
    let mut actual = findings
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
