use super::*;
use clap::Parser;
use std::ffi::OsString;

fn options(paths: &[PathBuf], explicit: Option<&Path>) -> Args {
    let mut values = vec![OsString::from("rrubocop")];
    if let Some(path) = explicit {
        values.extend([OsString::from("--config"), path.as_os_str().to_owned()]);
    }
    values.extend(paths.iter().map(|path| path.as_os_str().to_owned()));
    Args::parse_from(values)
}

fn project() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let app = root.path().join("app/example.rb");
    let nested = root.path().join("dev/spec/example.rb");
    std::fs::create_dir_all(app.parent().unwrap()).unwrap();
    std::fs::create_dir_all(nested.parent().unwrap()).unwrap();
    for path in [&app, &nested] {
        std::fs::write(path, "\"value\"\n").unwrap();
    }
    std::fs::write(root.path().join(".rubocop.yml"), "AllCops:\n  DisabledByDefault: true\n  NewCops: disable\nStyle/StringLiterals:\n  Enabled: true\n").unwrap();
    std::fs::write(
        nested.parent().unwrap().join(".rubocop.yml"),
        "inherit_from: ../../.rubocop.yml\nStyle/StringLiterals:\n  Enabled: false\n",
    )
    .unwrap();
    (root, app, nested)
}

fn enabled(config: &ResolvedConfig, path: &Path) -> bool {
    let scoped = config.effective_config_for_file(path);
    scoped
        .as_ref()
        .unwrap_or(config)
        .is_cop_enabled("Style/StringLiterals", path, &[], &[])
}

#[test]
fn multiple_operands_preserve_sibling_overrides_in_both_orders() {
    let (root, app, nested) = project();
    for paths in [
        vec![app.clone(), nested.clone()],
        vec![nested.clone(), app.clone()],
        vec![
            app.parent().unwrap().to_owned(),
            nested.parent().unwrap().to_owned(),
        ],
    ] {
        let config = resolved_config(&options(&paths, None)).unwrap();
        assert_eq!(config.config_dir(), Some(root.path()));
        assert!(enabled(&config, &app));
        assert!(!enabled(&config, &nested));
    }
}

#[test]
fn single_file_and_explicit_configuration_keep_existing_semantics() {
    let (root, app, nested) = project();
    let config = resolved_config(&options(std::slice::from_ref(&nested), None)).unwrap();
    assert_eq!(config.config_dir(), nested.parent());
    assert!(!enabled(&config, &nested));
    let config = resolved_config(&options(
        &[nested.clone(), app.clone()],
        Some(&root.path().join(".rubocop.yml")),
    ))
    .unwrap();
    assert!(enabled(&config, &app));
    assert!(enabled(&config, &nested));
}
