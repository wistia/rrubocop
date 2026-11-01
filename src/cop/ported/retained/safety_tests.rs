use super::*;
use crate::cli::AutocorrectMode;
use crate::config::{CopFilterSet, ResolvedConfig};
use crate::cop::{CopConfig, EnabledState};
use std::path::Path;

fn correct(
    source: &str,
    names: &[&str],
    mode: AutocorrectMode,
    grouped: bool,
) -> (String, Vec<crate::diagnostic::Diagnostic>) {
    let config = configuration(names, grouped);
    let registry = CopRegistry::default_registry();
    let only = selected(names);
    let mut bytes = source.as_bytes().to_vec();
    let diagnostics = crate::linter::lint_bytes_autocorrect(
        Path::new("probe.rb"),
        &mut bytes,
        &config,
        &registry,
        &CopFilterSet::build(&config, &registry),
        Some(&only),
        &[],
        mode,
        false,
    )
    .unwrap();
    (String::from_utf8(bytes).unwrap(), diagnostics)
}

fn selected(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).into()).collect()
}

fn configuration(names: &[&str], grouped: bool) -> ResolvedConfig {
    let mut config = ResolvedConfig::empty();
    config.target_ruby_version = Some(3.4);
    config.display_cop_names = false;
    for name in names {
        let mut cop = CopConfig {
            enabled: EnabledState::True,
            ..CopConfig::default()
        };
        if grouped && *name == "Style/MixinGrouping" {
            cop.options.insert("EnforcedStyle".into(), "grouped".into());
        }
        config.cop_configs.insert((*name).into(), cop);
    }
    config
}

#[test]
fn unsafe_reference_shapes_remain_manual_in_every_correction_mode() {
    let cases: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/retained-autocorrect-safety.json"
    )))
    .unwrap();
    for case in cases
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| case.get("rust_correctable") == Some(&serde_json::Value::Bool(false)))
    {
        assert_manual(case);
    }
}

fn assert_manual(case: &serde_json::Value) {
    let name = case["cop"].as_str().unwrap();
    let source = case["source"].as_str().unwrap();
    let grouped = case["options"]["EnforcedStyle"] == "grouped";
    for mode in [
        AutocorrectMode::Off,
        AutocorrectMode::Safe,
        AutocorrectMode::All,
    ] {
        let (after, diagnostics) = correct(source, &[name], mode, grouped);
        assert_eq!(after, source, "{name}: {source}");
        assert!(
            diagnostics.iter().all(|d| !d.correctable && !d.corrected),
            "{name}: {source}"
        );
    }
}

#[test]
fn all_mixin_grouping_rewrites_remain_manual() {
    for (source, grouped, count) in [
        ("class X\n include A, B\nend\n", false, 1),
        ("class X\n include A\n include B\nend\n", true, 2),
    ] {
        for mode in [
            AutocorrectMode::Off,
            AutocorrectMode::Safe,
            AutocorrectMode::All,
        ] {
            let (after, diagnostics) = correct(source, &["Style/MixinGrouping"], mode, grouped);
            assert_eq!(after, source);
            assert_eq!(diagnostics.len(), count);
            assert!(diagnostics.iter().all(|d| !d.correctable && !d.corrected));
        }
    }
    assert!(
        !CopRegistry::default_registry()
            .get("Style/MixinGrouping")
            .unwrap()
            .supports_autocorrect()
    );
}

#[test]
fn group_comments_and_disabled_members_cannot_leave_duplicate_inclusions() {
    let source = "class X\n include A\n include B # rubocop:disable Style/MixinGrouping\nend\n";
    let (after, diagnostics) =
        correct(source, &["Style/MixinGrouping"], AutocorrectMode::All, true);
    assert_eq!(after, source);
    assert_eq!(diagnostics.len(), 1);
    assert!(!diagnostics[0].correctable);
}

#[test]
fn ordinary_times_rewrites_keep_nested_control_targets_and_return_scope() {
    for (source, expected) in [
        (
            "1.times { items.each { break } }\n",
            "items.each { break }\n",
        ),
        (
            "def f\n 1.times { return 7 }\nend\n",
            "def f\n return 7\nend\n",
        ),
        ("1.times { work }; keep_me\n", "work; keep_me\n"),
    ] {
        assert_eq!(
            correct(source, &["Lint/UselessTimes"], AutocorrectMode::All, false).0,
            expected
        );
        assert_eq!(
            correct(source, &["Lint/UselessTimes"], AutocorrectMode::Safe, false).0,
            source
        );
    }
}

#[test]
fn unsafe_shape_offense_fixtures_are_exercised() {
    let registry = CopRegistry::default_registry();
    for (name, fixture) in [
        (
            "Lint/UselessTimes",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/lint/useless_times/unsafe_autocorrect.rb"
            ))
            .as_slice(),
        ),
        (
            "Style/MixinGrouping",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/mixin_grouping/unsafe_autocorrect.rb"
            ))
            .as_slice(),
        ),
    ] {
        crate::testutil::assert_cop_offenses_full(registry.get(name).unwrap(), fixture);
    }
}

#[test]
fn parameter_substitution_respects_nested_scopes_and_pattern_bindings() {
    let reference: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/retained-scope-hook-safety.json"
    )))
    .unwrap();
    for case in reference
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| case["rust_correctable"] == false)
    {
        assert_manual(case);
    }
}
