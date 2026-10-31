use super::*;
use crate::cop::{Cop, CopConfig, EnabledState};
use crate::parse::source::SourceFile;
use std::path::Path;

fn autocorrect(source: &str) -> Vec<Diagnostic> {
    let mut config = ResolvedConfig::empty();
    let names = ["Cops/NoSetex", "Cops/NoSetWithoutExpiration"];
    for name in names {
        config.cop_configs.insert(
            name.into(),
            CopConfig {
                enabled: EnabledState::True,
                ..CopConfig::default()
            },
        );
    }
    let registry = CopRegistry::default_registry();
    lint_bytes_autocorrect(
        Path::new("probe.rb"),
        &mut source.as_bytes().to_vec(),
        &config,
        &registry,
        &CopFilterSet::build(&config, &registry),
        Some(&names.map(str::to_owned)),
        &[],
        AutocorrectMode::Safe,
        false,
    )
    .unwrap()
}
#[test]
fn fully_corrected_real_diagnostics_return_success() {
    let diagnostics = autocorrect("redis.setex(key, 60, 'value')\n");
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].corrected);
    assert!(!should_fail(&diagnostics, "convention"));
    assert!(!should_fail(&diagnostics, "unknown-level"));
}
#[test]
fn a_remaining_offense_still_fails_after_another_is_corrected() {
    let diagnostics = autocorrect("redis.setex(key, 60, 'value')\nredis.set(other, 'value')\n");
    assert!(diagnostics.iter().any(|d| d.corrected));
    assert!(diagnostics.iter().any(|d| !d.corrected));
    assert!(should_fail(&diagnostics, "convention"));
    assert!(should_fail(&diagnostics, "unknown-level"));
}
fn diagnostic(severity: Severity, corrected: bool) -> Diagnostic {
    let mut diagnostic = crate::cop::lint::Syntax.diagnostic(
        &SourceFile::from_bytes("probe.rb", b"work\n".to_vec()),
        1,
        0,
        "test offense".into(),
    );
    diagnostic.severity = severity;
    diagnostic.corrected = corrected;
    diagnostic
}
#[test]
fn thresholds_use_only_the_severity_of_remaining_offenses() {
    let diagnostics = [
        diagnostic(Severity::Fatal, true),
        diagnostic(Severity::Warning, false),
    ];
    assert!(should_fail(&diagnostics, "convention"));
    assert!(should_fail(&diagnostics, "warning"));
    assert!(!should_fail(&diagnostics, "error"));
    assert!(!should_fail(&diagnostics, "fatal"));
}
#[test]
fn an_uncorrected_fatal_remains_a_failure_at_the_highest_threshold() {
    assert!(should_fail(&[diagnostic(Severity::Fatal, false)], "fatal"));
    assert!(!should_fail(&[], "unknown-level"));
}
