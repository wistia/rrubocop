use super::*;
use crate::cop::{CopConfig, EnabledState};

fn correct(
    source: &[u8],
    names: &[&str],
    mode: AutocorrectMode,
    ignore: bool,
) -> (Vec<u8>, Vec<Diagnostic>) {
    let config = enabled_config(names);
    let registry = CopRegistry::default_registry();
    let filters = CopFilterSet::build(&config, &registry);
    let mut bytes = source.to_vec();
    let only = names.iter().map(|name| (*name).into()).collect::<Vec<_>>();
    let diagnostics = lint_bytes_autocorrect(
        Path::new("probe.rb"),
        &mut bytes,
        &config,
        &registry,
        &filters,
        Some(&only),
        &[],
        mode,
        ignore,
    )
    .unwrap();
    (bytes, diagnostics)
}

fn enabled_config(names: &[&str]) -> ResolvedConfig {
    let mut config = ResolvedConfig::empty();
    config.target_ruby_version = Some(3.4);
    for &name in names {
        config.cop_configs.insert(
            name.into(),
            CopConfig {
                enabled: EnabledState::True,
                ..CopConfig::default()
            },
        );
    }
    config
}

#[test]
fn unicode_prefixes_do_not_drop_multiple_correction_owners() {
    let source =
        include_bytes!("../../../../tests/fixtures/engine/directives/unicode_whitespace.rb");
    for mode in [AutocorrectMode::Safe, AutocorrectMode::All] {
        let (after, diagnostics) = correct(source, &["Layout/TrailingWhitespace"], mode, false);
        assert_eq!(after, "éééé = 1\néééé = 2\n".as_bytes());
        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics.iter().all(|diagnostic| diagnostic.corrected));
    }
}

#[test]
fn fake_literal_directives_do_not_suppress_or_get_autocorrected() {
    let cases: &[&[u8]] = &[
        include_bytes!("../../../../tests/fixtures/engine/directives/heredoc_literal.rb"),
        include_bytes!("../../../../tests/fixtures/engine/directives/quoted_literal.rb"),
        include_bytes!("../../../../tests/fixtures/engine/directives/percent_literal.rb"),
        include_bytes!("../../../../tests/fixtures/engine/directives/regexp_literal.rb"),
        include_bytes!("../../../../tests/fixtures/engine/directives/block_comment.rb"),
        include_bytes!(
            "../../../../tests/fixtures/engine/directives/heredoc_redundant_directives.rb"
        ),
    ];
    let names = [
        "Layout/TrailingWhitespace",
        "Lint/RedundantCopDisableDirective",
        "Lint/RedundantCopEnableDirective",
    ];
    for &source in cases {
        for mode in [
            AutocorrectMode::Off,
            AutocorrectMode::Safe,
            AutocorrectMode::All,
        ] {
            assert_literal_correction(source, &names, mode);
        }
    }
}

fn assert_literal_correction(source: &[u8], names: &[&str], mode: AutocorrectMode) {
    let (after, diagnostics) = correct(source, names, mode, false);
    assert_eq!(
        diagnostics.len(),
        1,
        "{}: {diagnostics:?}",
        String::from_utf8_lossy(source)
    );
    assert_eq!(diagnostics[0].cop_name, "Layout/TrailingWhitespace");
    let expected = if mode == AutocorrectMode::Off {
        source.to_vec()
    } else {
        source
            .strip_suffix(b"  \n")
            .unwrap()
            .iter()
            .copied()
            .chain([b'\n'])
            .collect()
    };
    assert_eq!(after, expected);
    assert_eq!(diagnostics[0].corrected, mode != AutocorrectMode::Off);
}

#[test]
fn literal_prefix_on_real_directive_lines_cannot_change_audit_state_or_removals() {
    for mode in [AutocorrectMode::Safe, AutocorrectMode::All] {
        let source = b"puts \"# rubocop:disable Layout/TrailingWhitespace\" # rubocop:enable Layout/TrailingWhitespace\nx = 1  \n";
        let (after, diagnostics) = correct(
            source,
            &[
                "Layout/TrailingWhitespace",
                "Lint/RedundantCopDisableDirective",
                "Lint/RedundantCopEnableDirective",
            ],
            mode,
            false,
        );
        assert_eq!(after, b"puts \"# rubocop:disable Layout/TrailingWhitespace\" # rubocop:enable Layout/TrailingWhitespace\nx = 1\n");
        assert_eq!(diagnostics.len(), 1);
        let source = b"puts \"# rubocop:disable Layout/TrailingWhitespace\" # rubocop:disable Layout/TrailingWhitespace\nx = 1  \n";
        let (after, _) = correct(
            source,
            &[
                "Layout/TrailingWhitespace",
                "Lint/RedundantCopDisableDirective",
            ],
            mode,
            false,
        );
        assert_eq!(
            after,
            b"puts \"# rubocop:disable Layout/TrailingWhitespace\"\nx = 1\n"
        );
    }
}

#[test]
fn real_interpolation_directives_and_chained_comments_keep_their_scope() {
    for source in [
        "value = \"#{\n# rubocop:disable Layout/TrailingWhitespace\nx = 1  \n# rubocop:enable Layout/TrailingWhitespace\n}\"\nx = 2  \n",
        "x = 1 # rubocop:disable Layout/TrailingWhitespace # rubocop:todo Layout/TrailingWhitespace  \nx = 2  \n",
    ] {
        for mode in [AutocorrectMode::Safe, AutocorrectMode::All] {
            let (after, diagnostics) = correct(
                source.as_bytes(),
                &["Layout/TrailingWhitespace"],
                mode,
                false,
            );
            assert_eq!(
                after,
                source
                    .trim_end()
                    .as_bytes()
                    .iter()
                    .copied()
                    .chain([b'\n'])
                    .collect::<Vec<_>>()
            );
            assert_eq!(diagnostics.len(), 1);
            let (ignored, _) = correct(
                source.as_bytes(),
                &["Layout/TrailingWhitespace"],
                mode,
                true,
            );
            assert!(
                !String::from_utf8(ignored)
                    .unwrap()
                    .lines()
                    .any(|line| line.ends_with("  "))
            );
        }
    }
}

#[test]
fn repeated_insertion_edits_can_be_owned_by_the_end_of_the_highlight() {
    for source in ["foo(1,2); foo(1,2)\n", "puts \"#{foo(1,2);foo(1,2)}\"\n"] {
        for mode in [AutocorrectMode::Safe, AutocorrectMode::All] {
            let (after, diagnostics) =
                correct(source.as_bytes(), &["Layout/SpaceAfterComma"], mode, false);
            assert_eq!(after, source.replace(",2", ", 2").as_bytes());
            assert_eq!(diagnostics.len(), 2);
            assert!(diagnostics.iter().all(|diagnostic| diagnostic.corrected));
        }
    }
}

#[test]
fn single_redundant_directive_range_starts_at_actual_comment_marker() {
    let fixture = crate::testutil::parse_fixture(include_bytes!(
        "../../../../tests/fixtures/cops/lint/redundant_cop_disable_directive/offense_single_redundant.rb"
    ));
    for mode in [
        AutocorrectMode::Off,
        AutocorrectMode::Safe,
        AutocorrectMode::All,
    ] {
        let (after, diagnostics) = correct(
            &fixture.source,
            &[
                "Layout/TrailingWhitespace",
                "Lint/RedundantCopDisableDirective",
            ],
            mode,
            false,
        );
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].location.line, fixture.expected[0].line);
        assert_eq!(diagnostics[0].location.column, fixture.expected[0].column);
        assert_eq!(
            diagnostics[0].highlight_length,
            "# rubocop:disable Layout/TrailingWhitespace".len()
        );
        assert_eq!(diagnostics[0].corrected, mode != AutocorrectMode::Off);
        if mode == AutocorrectMode::Off {
            assert_eq!(after, fixture.source);
        } else {
            assert_eq!(
                after,
                b"puts \"# rubocop:disable Layout/TrailingWhitespace\"\n"
            );
        }
    }
}
