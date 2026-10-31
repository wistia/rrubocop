use super::*;
use crate::cop::{CopConfig, EnabledState};

fn correct(source: &str, mode: AutocorrectMode, ignore: bool) -> (Vec<u8>, Vec<Diagnostic>) {
    let mut config = ResolvedConfig::empty();
    config.cop_configs.insert(
        "Cops/NoSetex".into(),
        CopConfig {
            enabled: EnabledState::True,
            ..CopConfig::default()
        },
    );
    let registry = CopRegistry::default_registry();
    let filters = CopFilterSet::build(&config, &registry);
    let mut bytes = source.as_bytes().to_vec();
    let diagnostics = lint_bytes_autocorrect(
        Path::new("probe.rb"),
        &mut bytes,
        &config,
        &registry,
        &filters,
        Some(&["Cops/NoSetex".into()]),
        &[],
        mode,
        ignore,
    )
    .unwrap();
    assert!(diagnostics.iter().all(|d| d.cop_name == "Cops/NoSetex"));
    (bytes, diagnostics)
}
#[test]
fn disabled_offenses_preserve_all_their_edits_in_both_modes() {
    for mode in [AutocorrectMode::Safe, AutocorrectMode::All] {
        for source in [
            "redis.setex(key, 60, 'value') # rubocop:disable Cops/NoSetex\n",
            "redis.setex(key, 60, 'value') # rubocop:todo NoSetex\n",
            "# rubocop:disable Cops/NoSetex\nredis.setex(key, 60, 'value')\n# rubocop:enable Cops/NoSetex\n",
            "# rubocop:disable all\nredis.setex(key, 60, 'value')\n# rubocop:enable all\n",
            "redis.setex( # rubocop:disable Cops/NoSetex\n key,\n 60,\n 'value'\n)\n",
        ] {
            let (after, diagnostics) = correct(source, mode, false);
            assert_eq!(after, source.as_bytes(), "{source}");
            assert!(diagnostics.is_empty(), "{diagnostics:?}");
        }
    }
}
#[test]
fn mixed_disabled_and_enabled_calls_keep_separate_owners() {
    for mode in [AutocorrectMode::Safe, AutocorrectMode::All] {
        let (after, diagnostics) = correct(
            "redis.setex(key, 60, 'value') # rubocop:disable Cops/NoSetex\nredis.setex(other, 30, 'other')\n",
            mode,
            false,
        );
        assert_eq!(after, b"redis.setex(key, 60, 'value') # rubocop:disable Cops/NoSetex\nredis.set(other, 'other', ex: 30)\n");
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].corrected);
        assert_eq!(diagnostics[0].location.line, 2);
    }
}
#[test]
fn accepted_edits_outside_the_highlight_start_are_reported() {
    let (after, diagnostics) = correct(
        "redis.setex(key, 60, 'value')\n",
        AutocorrectMode::Safe,
        false,
    );
    assert_eq!(after, b"redis.set(key, 'value', ex: 60)\n");
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].corrected);
    assert_eq!(diagnostics[0].location.column, 0);
}
#[test]
fn ignore_disable_comments_applies_and_reports_the_correction() {
    let (after, diagnostics) = correct(
        "redis.setex(key, 60, 'value') # rubocop:disable all\n",
        AutocorrectMode::All,
        true,
    );
    assert_eq!(
        after,
        b"redis.set(key, 'value', ex: 60) # rubocop:disable all\n"
    );
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].corrected);
}

#[test]
fn legacy_source_cops_respect_mixed_disabled_and_enabled_lines() {
    let mut bytes = b"x = 1 # rubocop:disable Layout/TrailingWhitespace  \ny = 2  \n".to_vec();
    let diagnostics = super::tests::lint_only("probe.rb", &mut bytes, "Layout/TrailingWhitespace");
    assert_eq!(
        bytes,
        b"x = 1 # rubocop:disable Layout/TrailingWhitespace  \ny = 2\n"
    );
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].corrected);
    assert_eq!(diagnostics[0].location.line, 2);
}

#[test]
fn table_cops_fill_an_empty_options_hash_in_separate_passes() {
    let mut config = ResolvedConfig::empty();
    let names = [
        "Cops/CreateTableMustIncludeCharset",
        "Cops/CreateTableMustIncludeCollation",
    ];
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
    let filters = CopFilterSet::build(&config, &registry);
    let mut bytes = b"create_table(:widgets, {})\n".to_vec();
    let diagnostics = lint_bytes_autocorrect(
        Path::new("db/migrate/probe.rb"),
        &mut bytes,
        &config,
        &registry,
        &filters,
        Some(&names.map(str::to_owned)),
        &[],
        AutocorrectMode::Safe,
        false,
    )
    .unwrap();
    assert_eq!(
        bytes,
        b"create_table(:widgets, {charset: \"utf8mb4\", collation: \"utf8mb4_unicode_ci\"})\n"
    );
    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().all(|d| d.corrected));
}
