use super::*;
use crate::cop::{CopConfig, EnabledState};

#[test]
fn disabled_autocorrection_preserves_source_in_both_modes() {
    for mode in [AutocorrectMode::Safe, AutocorrectMode::All] {
        let (bytes, diagnostics) = correct("AutoCorrect", "disabled".into(), mode);
        assert_eq!(bytes, b"str.split(\"\")\n");
        assert_eq!(diagnostics.len(), 1);
        assert!(!diagnostics[0].corrected);
    }
}
#[test]
fn unsafe_configuration_requires_all_corrections_mode() {
    for key in ["Safe", "SafeAutoCorrect"] {
        let (bytes, diagnostics) = correct(key, false.into(), AutocorrectMode::Safe);
        assert_eq!(bytes, b"str.split(\"\")\n");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            correct(key, false.into(), AutocorrectMode::All).0,
            b"str.chars\n"
        );
    }
}
fn correct(
    key: &str,
    value: serde_yml::Value,
    mode: AutocorrectMode,
) -> (Vec<u8>, Vec<Diagnostic>) {
    let config = config(key, value);
    let registry = CopRegistry::default_registry();
    let filters = CopFilterSet::build(&config, &registry);
    let mut bytes = b"str.split(\"\")\n".to_vec();
    let diagnostics = lint_bytes_autocorrect(
        Path::new("t.rb"),
        &mut bytes,
        &config,
        &registry,
        &filters,
        Some(&["Style/StringChars".into()]),
        &[],
        mode,
        false,
    )
    .unwrap();
    assert!(
        diagnostics
            .iter()
            .all(|d| d.cop_name == "Style/StringChars")
    );
    (bytes, diagnostics)
}
fn config(key: &str, value: serde_yml::Value) -> ResolvedConfig {
    let mut config = ResolvedConfig::empty();
    config.cop_configs.insert(
        "Style/StringChars".into(),
        CopConfig {
            enabled: EnabledState::True,
            options: std::collections::HashMap::from([(key.into(), value)]),
            ..CopConfig::default()
        },
    );
    config
}
