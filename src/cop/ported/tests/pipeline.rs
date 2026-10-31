use super::*;
use crate::cli::AutocorrectMode;
use crate::config::{CopFilterSet, ResolvedConfig};
use crate::cop::EnabledState;

#[test]
fn all_retained_checks_run_through_the_real_syntax_and_scope_pipeline() {
    let registry = CopRegistry::default_registry();
    for case in cases() {
        assert_eq!(
            inspect(&registry, &case, AutocorrectMode::Off).0,
            runner::expected(&case),
            "{} {:?}",
            case.cop,
            case.source
        );
    }
}
#[test]
fn all_retained_checks_preserve_diagnostics_and_edits_in_correction_modes() {
    let registry = CopRegistry::default_registry();
    for case in cases() {
        for mode in [AutocorrectMode::Safe, AutocorrectMode::All] {
            let (actual, corrected) = inspect(&registry, &case, mode);
            assert_eq!(
                actual,
                runner::expected(&case),
                "{} {mode:?} {:?}",
                case.cop,
                case.source
            );
            assert_eq!(
                corrected,
                expected_correction(&registry, &case, mode),
                "{} {mode:?} {:?}",
                case.cop,
                case.source
            );
        }
    }
}
fn expected_correction<'a>(
    registry: &CopRegistry,
    case: &'a Case,
    mode: AutocorrectMode,
) -> &'a str {
    let cop = registry.get(&case.cop).unwrap();
    let cfg = CopConfig {
        options: case.config.clone(),
        ..CopConfig::default()
    };
    let safe =
        cfg.get_bool("Safe", true) && cfg.get_bool("SafeAutoCorrect", cop.safe_autocorrect());
    if cop.supports_autocorrect()
        && !cfg.autocorrect_disabled()
        && (mode == AutocorrectMode::All || safe)
    {
        runner::expected_correction(case)
    } else {
        &case.source
    }
}

fn inspect(registry: &CopRegistry, case: &Case, mode: AutocorrectMode) -> (Vec<String>, String) {
    let config = configuration(case);
    let source = SourceFile::from_bytes(
        case.path.replace(
            "$PROJECT",
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/ported_project"),
        ),
        case.source.as_bytes().to_vec(),
    );
    let result = crate::linter::lint_source(
        &source,
        &config,
        registry,
        &CopFilterSet::build(&config, registry),
        Some(&[case.cop.clone()]),
        &[],
        mode,
        true,
    )
    .unwrap();
    let corrected =
        crate::correction::CorrectionSet::from_vec(result.corrections.unwrap_or_default())
            .apply(source.as_bytes());
    (
        actual(result.diagnostics),
        String::from_utf8(corrected).unwrap(),
    )
}
fn configuration(case: &Case) -> ResolvedConfig {
    let mut config = ResolvedConfig::empty();
    config.display_cop_names = false;
    config.target_ruby_version = Some(3.4);
    config.cop_configs.insert(
        case.cop.clone(),
        CopConfig {
            enabled: EnabledState::True,
            include: vec!["**/*".into()],
            options: case.config.clone(),
            ..CopConfig::default()
        },
    );
    config
}
fn actual(findings: Vec<Diagnostic>) -> Vec<String> {
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
