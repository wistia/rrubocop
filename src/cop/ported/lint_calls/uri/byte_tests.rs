use crate::cli::AutocorrectMode;
use crate::config::{CopFilterSet, ResolvedConfig};
use crate::cop::{CopConfig, CopRegistry, EnabledState};
use crate::diagnostic::Diagnostic;
use std::path::Path;

const HEADER: &[u8] = b"# encoding: ASCII-8BIT\n";

fn run(source: &[u8], cop: &str, mode: AutocorrectMode) -> (Vec<u8>, Vec<Diagnostic>) {
    let registry = CopRegistry::default_registry();
    let config = configuration(cop);
    let mut bytes = source.to_vec();
    let diagnostics = crate::linter::lint_bytes_autocorrect(
        Path::new("probe.rb"),
        &mut bytes,
        &config,
        &registry,
        &CopFilterSet::build(&config, &registry),
        Some(&[cop.into()]),
        &[],
        mode,
        false,
    )
    .unwrap();
    (bytes, diagnostics)
}
fn configuration(name: &str) -> ResolvedConfig {
    let mut config = ResolvedConfig::empty();
    config.target_ruby_version = Some(3.4);
    let mut cop = CopConfig {
        enabled: EnabledState::True,
        ..CopConfig::default()
    };
    for key in ["AutoCorrect", "Safe", "SafeAutoCorrect"] {
        cop.options.insert(key.into(), true.into());
    }
    config.cop_configs.insert(name.into(), cop);
    config
}
fn check(cop: &str, before: &[u8], after: &[u8], correctable: bool) {
    let before = [HEADER, before].concat();
    let after = [HEADER, after].concat();
    for mode in [
        AutocorrectMode::Off,
        AutocorrectMode::Safe,
        AutocorrectMode::All,
    ] {
        let (actual, diagnostics) = run(&before, cop, mode);
        assert_eq!(
            actual,
            if mode == AutocorrectMode::Off {
                &before
            } else {
                &after
            }[..],
            "{cop} {mode:?}"
        );
        assert_eq!(diagnostics.len(), 1, "{cop}");
        assert_eq!(diagnostics[0].correctable, correctable, "{cop}");
    }
}

#[test]
fn uri_edits_keep_non_utf8_arguments_and_blocks_byte_for_byte() {
    for (before, after) in [
        (
            &b"p URI.regexp(\"\xff\")\n"[..],
            &b"p URI::DEFAULT_PARSER.make_regexp(\"\xff\")\n"[..],
        ),
        (
            b"p URI.regexp \"\xff\"\n",
            b"p URI::DEFAULT_PARSER.make_regexp(\"\xff\")\n",
        ),
        (
            b"p ::URI.regexp(\"\xff\") { :ignored }\n",
            b"p ::URI::DEFAULT_PARSER.make_regexp(\"\xff\") { :ignored }\n",
        ),
        (
            b"p URI.regexp([\"https\"]) { \"\xff\" }\n",
            b"p URI::DEFAULT_PARSER.make_regexp([\"https\"]) { \"\xff\" }\n",
        ),
        (
            b"p URI.regexp() { \"\xff\" }\n",
            b"p URI::DEFAULT_PARSER.make_regexp { \"\xff\" }\n",
        ),
        (
            b"p URI.regexp(\n# keep argument bytes\n\"\xff\", :ignored)\n",
            b"p URI::DEFAULT_PARSER.make_regexp(\"\xff\")\n",
        ),
    ] {
        check("Lint/UriRegexp", before, after, true);
    }
}

#[test]
fn shared_boundaries_handle_non_utf8_receivers_and_ignored_blocks() {
    for (cop, before, after) in [
        (
            "Style/ClassEqualityComparison",
            &b"p \"\xff\".class == String\n"[..],
            &b"p \"\xff\".instance_of?(String)\n"[..],
        ),
        (
            "Style/ClassEqualityComparison",
            b"p object.class.eql?(String) { \"\xff\" }\n",
            b"p object.instance_of?(String) { \"\xff\" }\n",
        ),
        (
            "Style/HashConversion",
            b"consume(Hash[:a, 1], \"\xff\")\n",
            b"consume({:a => 1}, \"\xff\")\n",
        ),
        (
            "Style/UnpackFirst",
            b"p str.unpack(\"C*\").first { \"\xff\" }\n",
            b"p str.unpack1(\"C*\") { \"\xff\" }\n",
        ),
        (
            "Style/RedundantSort",
            b"p [\"\xff\"].sort.first\n",
            b"p [\"\xff\"].min\n",
        ),
    ] {
        check(cop, before, after, true);
    }
}

#[test]
fn string_reconstruction_keeps_non_utf8_edit_spans_manual() {
    for (cop, source) in [
        (
            "Style/ClassEqualityComparison",
            &b"p object.class.name == \"\xff\"\n"[..],
        ),
        ("Style/HashConversion", b"p Hash[\"\xff\", 1]\n"),
        ("Style/HashConversion", b"p Hash[[\"\xff\"].zip]\n"),
        ("Style/UnpackFirst", b"p str.unpack(\"\xff\").first\n"),
        ("Style/RedundantSort", b"p [1].sort_by { \"\xff\" }.first\n"),
    ] {
        check(cop, source, source, false);
    }
}

#[test]
fn class_safe_navigation_preserves_nil_comparison_behavior() {
    for source in [
        &b"object = nil; p object&.class == NilClass\n"[..],
        b"object = nil; p object&.class.name == \"NilClass\"\n",
        b"object = nil; p object.class&.name == \"NilClass\"\n",
        b"object = nil; p object.class&.eql?(NilClass)\n",
    ] {
        for mode in [
            AutocorrectMode::Off,
            AutocorrectMode::Safe,
            AutocorrectMode::All,
        ] {
            let (bytes, diagnostics) = run(source, "Style/ClassEqualityComparison", mode);
            assert_eq!(bytes, source);
            assert!(diagnostics.is_empty(), "{source:?}");
        }
    }
}
