use super::*;
use clap::Parser;
use std::path::Path;

fn arguments(path: &Path, force: bool, correct: bool) -> Args {
    let mut args = vec![
        "rrubocop".to_owned(),
        "--stdin".into(),
        path.to_string_lossy().into_owned(),
        "--only".into(),
        "Layout/SpaceAroundOperators".into(),
    ];
    if force {
        args.push("--force-exclusion".into());
    }
    if correct {
        args.push("-A".into());
    }
    Args::try_parse_from(args).unwrap()
}
fn configuration(root: &Path) -> ResolvedConfig {
    let mut config = ResolvedConfig::empty();
    config.config_dir = Some(root.to_path_buf());
    config.global_excludes = vec!["excluded.rb".into()];
    config.cop_configs.insert(
        "Layout/SpaceAroundOperators".into(),
        crate::cop::CopConfig {
            enabled: crate::cop::EnabledState::True,
            include: vec!["**/*.rb".into()],
            ..crate::cop::CopConfig::default()
        },
    );
    config
}
#[test]
fn force_excluded_stdin_never_corrects_an_existing_file() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("excluded.rb");
    let original = b"1+2\n";
    std::fs::write(&path, original).unwrap();
    let mut input = original.to_vec();
    let result = lint_stdin_source(
        &arguments(&path, true, true),
        &configuration(directory.path()),
        &CopRegistry::default_registry(),
        &path,
        &mut input,
    )
    .unwrap();
    assert!(result.is_none());
    assert_eq!(input, original);
    assert_eq!(std::fs::read(&path).unwrap(), original);
}
#[test]
fn explicitly_named_stdin_is_checked_without_force_exclusion() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("excluded.rb");
    let result = lint_stdin_source(
        &arguments(&path, false, false),
        &configuration(directory.path()),
        &CopRegistry::default_registry(),
        &path,
        &mut b"1+2\n".to_vec(),
    )
    .unwrap()
    .unwrap();
    assert!(!result.is_empty());
    assert!(
        result
            .iter()
            .all(|d| d.cop_name == "Layout/SpaceAroundOperators")
    );
}
#[test]
fn force_exclusion_does_not_skip_allowed_stdin() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("allowed.rb");
    let result = lint_stdin_source(
        &arguments(&path, true, false),
        &configuration(directory.path()),
        &CopRegistry::default_registry(),
        &path,
        &mut b"1+2\n".to_vec(),
    )
    .unwrap()
    .unwrap();
    assert!(!result.is_empty());
}
