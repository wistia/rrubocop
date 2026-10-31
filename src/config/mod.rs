//! RuboCop-compatible `.rubocop.yml` resolution (ported from nitrocop).
//!
//! Supports `inherit_from`, `inherit_gem`, `require`/`plugins`, `inherit_mode`,
//! nested configs, `DisabledByDefault`, and `NewCops` / `Enabled: pending`.

mod gem_configs;
pub mod gem_path;
mod gem_path_local;
mod gem_path_version;
mod gem_path_version_override;

mod discover;
mod filter;
mod filter_match;
mod filter_path;
mod globutil;
mod load;
mod load_defaults;
mod load_gems;
mod load_lockfile;
mod load_recursive;
mod load_require;
mod load_resolve;
mod merge;
mod merge_cop;
mod parse;
mod parse_allcops;
mod parse_cop;
mod rails_app;
mod resolved;
mod resolved_cop_cfg;
mod resolved_effective;
mod resolved_enabled;
mod resolved_filters;
mod resolved_fingerprint;
mod resolved_inject;
mod resolved_state;
mod resolved_versions;
mod ruby_ver;
mod standard;
mod standard_convert;
mod types;
mod yaml_read;

pub use filter::{CopFilter, CopFilterSet};
pub(crate) use gem_path_version_override::set_rubocop_version_override;
pub use gem_path_version_override::validate_rubocop_version;
pub use load::{load_config, load_default_config};
pub use resolved::ResolvedConfig;
pub use types::NewCopsPolicy;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cop::EnabledState;
    use std::fs;
    use std::path::{Path, PathBuf};

    fn write_yaml(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content).unwrap();
        path
    }

    fn write_config(dir: &Path, content: &str) -> PathBuf {
        write_yaml(dir, ".rubocop.yml", content)
    }

    fn fixtures_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/config")
    }

    #[test]
    fn legacy_project_exclude_replaces_required_plugin_excludes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Gemfile.lock"),
            "GEM\n  specs:\n    rubocop (1.65.1)\n    rubocop-rails (2.26.0)\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join(".rubocop.yml"),
            "require: rubocop-rails\nAllCops:\n  Exclude: [ignored/**/*]\n",
        )
        .unwrap();
        let config = super::load_config(None, Some(dir.path()), None).unwrap();
        assert!(!config.global_excludes.iter().any(|p| p == "bin/*"));
        assert!(config.global_excludes.iter().any(|p| p.contains("ignored")));
    }

    #[test]
    fn inherit_from_single_file() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(
            dir.path(),
            "base.yml",
            "Layout/LineLength:\n  Max: 100\nStyle/Foo:\n  Enabled: true\n",
        );
        let config = load_config(
            Some(&write_yaml(
                dir.path(),
                ".rubocop.yml",
                "inherit_from: base.yml\nLayout/LineLength:\n  Max: 120\n",
            )),
            None,
            None,
        )
        .unwrap();
        let cc = config.cop_config("Layout/LineLength");
        assert_eq!(cc.options.get("Max").and_then(|v| v.as_u64()), Some(120));
        assert!(config.is_cop_enabled("Style/Foo", Path::new("a.rb"), &[], &[]));
    }

    #[test]
    fn inherit_from_child_overrides_base() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "base.yml", "Style/Foo:\n  Enabled: true\n");
        let config = load_config(
            Some(&write_yaml(
                dir.path(),
                ".rubocop.yml",
                "inherit_from: base.yml\nStyle/Foo:\n  Enabled: false\n",
            )),
            None,
            None,
        )
        .unwrap();
        assert!(!config.is_cop_enabled("Style/Foo", Path::new("a.rb"), &[], &[]));
    }

    #[test]
    fn inherit_from_exclude_appends() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(
            dir.path(),
            "base.yml",
            "Style/Foo:\n  Exclude:\n    - 'vendor/**'\n",
        );
        let cc = load_config(
            Some(&write_yaml(
                dir.path(),
                ".rubocop.yml",
                "inherit_from: base.yml\nStyle/Foo:\n  Exclude:\n    - 'tmp/**'\n",
            )),
            None,
            None,
        )
        .unwrap()
        .cop_config("Style/Foo");
        assert!(cc.exclude.contains(&"vendor/**".to_string()));
        assert!(cc.exclude.contains(&"tmp/**".to_string()));
    }

    #[test]
    fn inherit_from_include_replaces() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(
            dir.path(),
            "base.yml",
            "Style/Foo:\n  Include:\n    - '**/*.rb'\n",
        );
        let config = load_config(
            Some(&write_yaml(
                dir.path(),
                ".rubocop.yml",
                "inherit_from: base.yml\nStyle/Foo:\n  Include:\n    - 'app/**'\n",
            )),
            None,
            None,
        )
        .unwrap();
        assert_eq!(
            config.cop_config("Style/Foo").include,
            vec!["app/**".to_string()]
        );
    }

    #[test]
    fn allcops_exclude_replaces_by_default() {
        // RuboCop replaces AllCops.Exclude unless inherit_mode.merge lists it.
        let dir = tempfile::tempdir().unwrap();
        write_yaml(
            dir.path(),
            "parent.yml",
            "AllCops:\n  Exclude:\n    - 'vendor/**/*'\nLayout/TrailingWhitespace:\n  Enabled: true\n",
        );
        let excludes = load_config(
            Some(&write_yaml(
                dir.path(),
                ".rubocop.yml",
                "inherit_from: parent.yml\nAllCops:\n  Exclude:\n    - 'tmp/**/*'\n",
            )),
            None,
            None,
        )
        .unwrap()
        .global_excludes()
        .to_vec();
        assert!(
            !excludes.iter().any(|e| e == "vendor/**/*"),
            "parent Exclude should be replaced: {excludes:?}"
        );
        assert!(
            excludes.iter().any(|e| e == "tmp/**/*"),
            "child Exclude should apply: {excludes:?}"
        );
    }

    #[test]
    fn allcops_exclude_merges_with_inherit_mode() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(
            dir.path(),
            "parent.yml",
            "inherit_mode:\n  merge:\n    - Exclude\nAllCops:\n  Exclude:\n    - 'vendor/**/*'\n",
        );
        let excludes = load_config(
            Some(&write_yaml(
                dir.path(),
                ".rubocop.yml",
                "inherit_from: parent.yml\nAllCops:\n  Exclude:\n    - 'tmp/**/*'\n",
            )),
            None,
            None,
        )
        .unwrap()
        .global_excludes()
        .to_vec();
        assert!(
            excludes.iter().any(|e| e == "vendor/**/*"),
            "parent Exclude should remain: {excludes:?}"
        );
        assert!(
            excludes.iter().any(|e| e == "tmp/**/*"),
            "child Exclude should apply: {excludes:?}"
        );
    }

    #[test]
    fn allcops_exclude_override_replaces() {
        // Override replaces inherit_from Exclude only; RuboCop built-in defaults
        // (vendor/**/*, …) remain. Use a custom pattern so defaults don't mask it.
        let dir = tempfile::tempdir().unwrap();
        write_yaml(
            dir.path(),
            "parent.yml",
            "AllCops:\n  Exclude:\n    - 'spec/fixtures/**/*'\n",
        );
        let excludes = load_config(
            Some(&write_yaml(
                dir.path(),
                ".rubocop.yml",
                "inherit_from: parent.yml\ninherit_mode:\n  override:\n    - Exclude\nAllCops:\n  Exclude:\n    - 'coverage/**/*'\n",
            )),
            None,
            None,
        )
        .unwrap()
        .global_excludes()
        .to_vec();
        assert!(
            !excludes.iter().any(|e| e == "spec/fixtures/**/*"),
            "inherited custom exclude should be replaced: {excludes:?}"
        );
        assert!(
            excludes.iter().any(|e| e == "coverage/**/*"),
            "local coverage exclude missing: {excludes:?}"
        );
    }

    #[test]
    fn inherit_mode_merge_include() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(
            dir.path(),
            "base.yml",
            "Style/Foo:\n  Include:\n    - '**/*.rb'\n",
        );
        let cc = load_config(
            Some(&write_yaml(
                dir.path(),
                ".rubocop.yml",
                "inherit_from: base.yml\ninherit_mode:\n  merge:\n    - Include\nStyle/Foo:\n  Include:\n    - '**/*.rake'\n",
            )),
            None,
            None,
        )
        .unwrap()
        .cop_config("Style/Foo");
        assert!(cc.include.contains(&"**/*.rb".to_string()));
        assert!(cc.include.contains(&"**/*.rake".to_string()));
    }

    #[test]
    fn inherit_mode_override_exclude() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(
            dir.path(),
            "base.yml",
            "Style/Foo:\n  Exclude:\n    - 'vendor/**'\n",
        );
        let cc = load_config(
            Some(&write_yaml(
                dir.path(),
                ".rubocop.yml",
                "inherit_from: base.yml\ninherit_mode:\n  override:\n    - Exclude\nStyle/Foo:\n  Exclude:\n    - 'tmp/**'\n",
            )),
            None,
            None,
        )
        .unwrap()
        .cop_config("Style/Foo");
        assert!(!cc.exclude.contains(&"vendor/**".to_string()));
        assert!(cc.exclude.contains(&"tmp/**".to_string()));
    }

    #[test]
    fn disabled_by_default_disables_unset_cops() {
        let config = load_config(
            Some(&write_config(
                tempfile::tempdir().unwrap().path(),
                "AllCops:\n  DisabledByDefault: true\nStyle/Foo:\n  Enabled: true\n",
            )),
            None,
            None,
        )
        .unwrap();
        assert!(config.is_cop_enabled("Style/Foo", Path::new("a.rb"), &[], &[]));
        assert!(!config.is_cop_enabled("Style/Bar", Path::new("a.rb"), &[], &[]));
    }

    #[test]
    fn circular_inherit_from_breaks_cycle() {
        let result = load_config(
            Some(&fixtures_dir().join("inherit_from/circular_a.yml")),
            None,
            None,
        );
        assert!(result.is_ok(), "cycle should be broken: {result:?}");
    }

    #[test]
    fn inherit_gem_unknown_gem_errors() {
        // RuboCop raises Gem::LoadError; we must not warn-and-continue.
        let dir = tempfile::tempdir().unwrap();
        let err = load_config(
            Some(&write_config(
                dir.path(),
                "inherit_gem:\n  definitely_not_a_real_gem_xyz:\n    - config.yml\n",
            )),
            Some(dir.path()),
            None,
        )
        .unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("Unable to find gem definitely_not_a_real_gem_xyz"),
            "{msg}"
        );
    }

    #[test]
    fn inherit_from_nested_unknown_inherit_gem_errors() {
        // Nested inherit_gem must not be swallowed as inherit_from warning.
        let dir = tempfile::tempdir().unwrap();
        write_yaml(
            dir.path(),
            "base.yml",
            "inherit_gem:\n  definitely_not_a_real_gem_xyz:\n    - config.yml\n",
        );
        let err = load_config(
            Some(&write_config(dir.path(), "inherit_from: base.yml\n")),
            Some(dir.path()),
            None,
        )
        .unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("Unable to find gem definitely_not_a_real_gem_xyz"),
            "{msg}"
        );
    }

    #[test]
    fn inherit_gem_vendored_needs_no_install() {
        // Known gem: use embedded YAML; no Gemfile.lock / bundle install.
        let dir = tempfile::tempdir().unwrap();
        let excludes = load_config(
            Some(&write_config(
                dir.path(),
                "inherit_gem:\n  rubocop-rails:\n    - config/default.yml\n",
            )),
            Some(dir.path()),
            None,
        )
        .expect("vendored inherit_gem must load without gem install")
        .global_excludes()
        .to_vec();
        assert!(
            excludes.iter().any(|e| e.contains("schema.rb")),
            "expected Rails AllCops.Exclude from vendored config, got {excludes:?}"
        );
    }

    #[test]
    fn inherit_gem_omakase_vendored_needs_no_install() {
        // rails/rubocop-rails-omakase: file is `rubocop.yml` (not config/default.yml)
        // and pulls in nested plugins:/require: (rubocop-performance, rubocop-rails).
        let dir = tempfile::tempdir().unwrap();
        assert_omakase_applied(
            &load_config(
                Some(&write_config(
                    dir.path(),
                    "inherit_gem:\n  rubocop-rails-omakase:\n    - rubocop.yml\n",
                )),
                Some(dir.path()),
                None,
            )
            .expect("vendored omakase inherit_gem must load without gem install"),
        );
    }

    // Omakase disables whole departments and re-enables select cops.
    fn assert_omakase_applied(config: &ResolvedConfig) {
        assert!(config.is_cop_enabled("Layout/SpaceInsideParens", Path::new("a.rb"), &[], &[]));
        assert!(!config.is_cop_enabled("Style/Documentation", Path::new("a.rb"), &[], &[]));
    }

    #[test]
    fn rails_app_without_config_defaults_to_omakase() {
        // `rails new` (7.2+) ships no `.rubocop.yml`; the omakase style
        // should still apply out of the box, matching what a fresh app gets
        // after `bundle binstubs rubocop`.
        discover::with_user_config_suppressed(|| {
            let dir = tempfile::tempdir().unwrap();
            fs::create_dir_all(dir.path().join("config")).unwrap();
            fs::write(dir.path().join("config/application.rb"), "").unwrap();
            assert_omakase_applied(
                &load_config(None, Some(dir.path()), None)
                    .expect("rails app with no .rubocop.yml must default to omakase"),
            );
        });
    }

    #[test]
    fn non_rails_project_without_config_uses_rubocop_defaults() {
        // No `config/application.rb` / `bin/rails` → plain RuboCop defaults,
        // not omakase.
        discover::with_user_config_suppressed(|| {
            let config =
                load_config(None, Some(tempfile::tempdir().unwrap().path()), None).unwrap();
            assert!(config.is_cop_enabled("Style/Documentation", Path::new("a.rb"), &[], &[]));
        });
    }

    #[test]
    fn diamond_dependency_loads() {
        let config = load_config(
            Some(&fixtures_dir().join("inherit_from/diamond_root.yml")),
            None,
            None,
        )
        .unwrap();
        assert!(!config.is_cop_enabled("Style/Foo", Path::new("a.rb"), &[], &[]));
        assert!(config.is_cop_enabled(
            "Style/FrozenStringLiteralComment",
            Path::new("a.rb"),
            &[],
            &[]
        ));
        assert!(config.is_cop_enabled("Style/StringLiterals", Path::new("a.rb"), &[], &[]));
    }

    #[test]
    fn fixture_inherit_from_merges() {
        let config = load_config(
            Some(&fixtures_dir().join("inherit_from/child.yml")),
            None,
            None,
        )
        .unwrap();
        assert_eq!(
            config
                .cop_config("Layout/LineLength")
                .options
                .get("Max")
                .and_then(|v| v.as_u64()),
            Some(120)
        );
        let excludes = config.global_excludes();
        assert!(excludes.contains(&"vendor/**".to_string()));
        assert!(excludes.contains(&"tmp/**".to_string()));
    }

    fn assert_nested_cop_state(config: &ResolvedConfig, dir: &Path) {
        assert!(config.has_dir_overrides());
        assert_eq!(config.cop_config("Style/Foo").enabled, EnabledState::True);
        let nested = config.cop_config_for_file("Style/Foo", &dir.join("spec/a.rb"));
        assert_eq!(nested.enabled, EnabledState::False);
    }

    #[test]
    fn nested_dir_override_disables_cop() {
        let dir = tempfile::tempdir().unwrap();
        write_config(dir.path(), "Style/Foo:\n  Enabled: true\n");
        write_yaml(
            dir.path(),
            "spec/.rubocop.yml",
            "Style/Foo:\n  Enabled: false\n",
        );
        assert_nested_cop_state(
            &load_config(None, Some(dir.path()), None).unwrap(),
            dir.path(),
        );
    }

    fn parent_config_nested_fixture() -> (tempfile::TempDir, PathBuf) {
        let home = tempfile::tempdir().unwrap();
        let project = home.path().join("project");
        fs::create_dir_all(&project).unwrap();
        write_config(home.path(), "Style/Foo:\n  Enabled: true\n");
        write_yaml(
            &project,
            "spec/.rubocop.yml",
            "Style/Foo:\n  Enabled: false\n",
        );
        write_yaml(
            home.path(),
            "sibling/.rubocop.yml",
            "Style/Foo:\n  Enabled: false\n",
        );
        (home, project)
    }

    fn assert_nested_overrides_scoped_to_scan_root(config: &ResolvedConfig) {
        assert!(
            config
                .dir_overrides
                .iter()
                .any(|(d, _)| d.ends_with("spec"))
        );
        assert!(
            !config
                .dir_overrides
                .iter()
                .any(|(d, _)| d.ends_with("sibling"))
        );
        assert_eq!(
            config
                .cop_config_for_file("Style/Foo", Path::new("spec/a.rb"))
                .enabled,
            EnabledState::False
        );
    }

    #[test]
    fn nested_overrides_scoped_to_scan_root_not_config_dir() {
        let (_home, project) = parent_config_nested_fixture();
        assert_nested_overrides_scoped_to_scan_root(
            &load_config(None, Some(&project), None).unwrap(),
        );
    }

    #[test]
    fn cache_fingerprint_changes_with_config() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_a = load_config(
            Some(&write_config(dir.path(), "Style/Foo:\n  Enabled: true\n")),
            None,
            None,
        )
        .unwrap();
        let cfg_b = load_config(
            Some(&write_config(dir.path(), "Style/Foo:\n  Enabled: false\n")),
            None,
            None,
        )
        .unwrap();
        assert_ne!(cfg_a.cache_fingerprint(), cfg_b.cache_fingerprint());
    }

    #[test]
    fn cache_fingerprint_stable_across_config_reloads() {
        const CFG: &str = "Metrics/MethodLength:\n  Max: 10\n  CountAsOne: array\n";
        let (a, b) = {
            let dir = tempfile::tempdir().unwrap();
            (
                load_config(Some(&write_config(dir.path(), CFG)), None, None).unwrap(),
                load_config(Some(&write_config(dir.path(), CFG)), None, None).unwrap(),
            )
        };
        assert_eq!(a.cache_fingerprint(), b.cache_fingerprint());
    }

    fn scan_root_project(home: &Path, name: &str) -> PathBuf {
        let project = home.join(name);
        fs::create_dir_all(&project).unwrap();
        project
    }

    #[test]
    fn cache_fingerprint_differs_by_scan_root() {
        let home = tempfile::tempdir().unwrap();
        let project_a = scan_root_project(home.path(), "a");
        let project_b = scan_root_project(home.path(), "b");
        write_config(home.path(), "Style/Foo:\n  Enabled: true\n");
        write_yaml(
            &project_a,
            "spec/.rubocop.yml",
            "Style/Foo:\n  Enabled: false\n",
        );
        let cfg_a = load_config(None, Some(&project_a), None).unwrap();
        let cfg_b = load_config(None, Some(&project_b), None).unwrap();
        assert_ne!(cfg_a.cache_fingerprint(), cfg_b.cache_fingerprint());
    }

    #[test]
    fn rails_guides_nested_disables_redundant_percent_q() {
        let root = std::path::Path::new("/tmp/parity/rails");
        if !root.join(".rubocop.yml").exists() {
            return;
        }
        let config = load_config(None, Some(root), None).unwrap();
        assert!(
            config.has_dir_overrides(),
            "expected guides/.rubocop.yml override"
        );
        let abs = root.join("guides/test/epub_test.rb");
        let rel = std::path::Path::new("guides/test/epub_test.rb");
        assert!(
            config.disabled_by_dir_override("Style/RedundantPercentQ", &abs),
            "absolute guides path should disable Style/RedundantPercentQ"
        );
        assert!(
            config.disabled_by_dir_override("Style/RedundantPercentQ", rel),
            "relative guides path should disable Style/RedundantPercentQ"
        );
    }

    #[test]
    fn puma_like_disabled_by_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_config(
            dir.path(),
            "plugins: rubocop-performance\n\nAllCops:\n  DisabledByDefault: true\n  NewCops: enable\n\nPerformance:\n  Enabled: true\n\nLayout/SpaceBeforeBlockBraces:\n  Enabled: true\n",
        );
        let config = load_config(Some(&path), Some(dir.path()), None).unwrap();
        assert!(config.disabled_by_default);
        assert!(config.is_cop_enabled(
            "Layout/SpaceBeforeBlockBraces",
            std::path::Path::new("a.rb"),
            &[],
            &[]
        ));
        assert!(!config.is_cop_enabled(
            "Style/RedundantConstantBase",
            std::path::Path::new("a.rb"),
            &[],
            &[]
        ));
    }

    #[test]
    fn force_default_respects_enabled_false() {
        let config = load_default_config(None, None);
        let path = std::path::Path::new("a.rb");
        assert!(
            !config.is_cop_enabled("Layout/MultilineArrayLineBreaks", path, &[], &[]),
            "Enabled: false in default.yml must stay off under --force-default-config"
        );
        assert!(
            !config.is_cop_enabled("Layout/MultilineMethodArgumentLineBreaks", path, &[], &[]),
            "Enabled: false in default.yml must stay off under --force-default-config"
        );
        assert!(
            config.is_cop_enabled("Layout/ArrayAlignment", path, &[], &[]),
            "Enabled: true in default.yml must stay on under --force-default-config"
        );
    }

    #[test]
    fn style_department_disabled_keeps_explicit_cops() {
        // Mirrors ~/.rubocop.yml-style: dept off, selected cops still on.
        let dir = tempfile::tempdir().unwrap();
        let config = load_config(
            Some(&write_config(
                dir.path(),
                "Layout/LineLength:\n  Enabled: true\n  Max: 120\n\
                 Lint/UselessAssignment:\n  Enabled: true\n\
                 Style:\n  Enabled: false\n\
                 Layout:\n  Enabled: false\n\
                 Metrics:\n  Enabled: false\n",
            )),
            Some(dir.path()),
            None,
        )
        .unwrap();
        assert!(!config.is_cop_enabled(
            "Style/FrozenStringLiteralComment",
            Path::new("x.rb"),
            &[],
            &[]
        ));
        assert!(config.is_cop_enabled("Lint/UselessAssignment", Path::new("x.rb"), &[], &[]));
        assert!(config.is_cop_enabled("Layout/LineLength", Path::new("x.rb"), &[], &[]));
    }
}
