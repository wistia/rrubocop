use super::*;
use crate::diagnostic::{Location, Severity};
use std::sync::Arc;

fn diagnostic(column: usize) -> Diagnostic {
    Diagnostic {
        path: "probe.rb".into(),
        location: Location { line: 1, column },
        severity: Severity::Warning,
        cop_name: "Test/Cop".into(),
        message: "message".into(),
        corrected: false,
        correctable: true,
        source_line: Default::default(),
        highlight_length: 1,
    }
}

#[test]
fn source_lines_share_one_snapshot_and_survive_corrected_source_replacement() {
    let source = SourceFile::from_bytes("probe.rb", "é old\r\n".as_bytes().to_vec());
    let mut first = diagnostic(0);
    let mut second = diagnostic(2);
    fill_source_highlight(&source, &mut first);
    fill_source_highlight(&source, &mut second);
    assert!(Arc::ptr_eq(&first.source_line, &second.source_line));
    let clone = first.clone();
    assert!(Arc::ptr_eq(&clone.source_line, &first.source_line));
    drop(source);
    assert_next_snapshot(&first);
}
fn assert_next_snapshot(first: &Diagnostic) {
    let corrected = SourceFile::from_bytes("probe.rb", b"new\n".to_vec());
    let mut later = diagnostic(0);
    fill_source_highlight(&corrected, &mut later);
    assert_eq!(first.source_line.as_ref(), "é old");
    assert_eq!(later.source_line.as_ref(), "new");
    assert!(!Arc::ptr_eq(&first.source_line, &later.source_line));
    assert_serde(first);
}
fn assert_serde(first: &Diagnostic) {
    let encoded = serde_json::to_value(first).unwrap();
    assert_eq!(encoded["source_line"], "é old");
    let restored: Diagnostic = serde_json::from_value(encoded).unwrap();
    assert_eq!(restored.source_line.as_ref(), "é old");
    assert!(
        serde_json::to_value(diagnostic(0))
            .unwrap()
            .get("source_line")
            .is_none()
    );
}

#[test]
fn actual_autocorrection_retains_original_excerpt_after_rewriting_bytes() {
    let (config, registry) = correction_fixture();
    let filters = crate::config::CopFilterSet::build(&config, &registry);
    let mut bytes = b"x = 1  \n".to_vec();
    let diagnostics = crate::linter::lint_bytes_autocorrect(
        std::path::Path::new("probe.rb"),
        &mut bytes,
        &config,
        &registry,
        &filters,
        Some(&["Layout/TrailingWhitespace".into()]),
        &[],
        crate::cli::AutocorrectMode::Safe,
        false,
    )
    .unwrap();
    assert_eq!(bytes, b"x = 1\n");
    assert_eq!(diagnostics[0].source_line.as_ref(), "x = 1  ");
    assert!(diagnostics[0].corrected);
}
fn correction_fixture() -> (ResolvedConfig, CopRegistry) {
    let mut config = ResolvedConfig::empty();
    config.cop_configs.insert(
        "Layout/TrailingWhitespace".into(),
        CopConfig {
            enabled: crate::cop::EnabledState::True,
            ..CopConfig::default()
        },
    );
    (config, CopRegistry::default_registry())
}
