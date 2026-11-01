use super::*;
use crate::config::load_config;

fn prepared<'a>(
    config: &'a ResolvedConfig,
    registry: &'a CopRegistry,
    filters: &'a CopFilterSet,
    configs: &'a PreparedConfigs,
) -> PreparedLint<'a> {
    PreparedLint {
        config,
        registry,
        filters,
        configs,
    }
}

#[test]
fn root_configs_are_prepared_only_on_demand_and_reused() {
    let config = ResolvedConfig::empty();
    let registry = CopRegistry::default_registry();
    let filters = CopFilterSet::build(&config, &registry);
    let configs = PreparedConfigs::new(&config, &registry);
    let context = prepared(&config, &registry, &filters, &configs);
    assert!(configs.root.configs.iter().all(|slot| slot.get().is_none()));
    assert_shared_root(context);
    assert_eq!(
        configs
            .root
            .configs
            .iter()
            .filter(|slot| slot.get().is_some())
            .count(),
        1
    );
}

fn assert_shared_root(context: PreparedLint<'_>) {
    let first = context.for_file(Path::new("example.rb")).get(0);
    let next = context.for_file(Path::new("other.rb")).get(0);
    assert!(std::ptr::eq(first, next));
}

fn project() -> tempfile::TempDir {
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(project.path().join("nested/deeper")).unwrap();
    for (path, yaml) in [
        (
            ".rubocop.yml",
            "AllCops:\n  TargetRubyVersion: 3.1\nLayout/IndentationWidth:\n  Width: 2\nLayout/LineLength:\n  Max: 100\nStyle/StringLiterals:\n  EnforcedStyle: single_quotes\n",
        ),
        (
            "nested/.rubocop.yml",
            "inherit_from: ../.rubocop.yml\nAllCops:\n  TargetRubyVersion: 3.2\nLayout/IndentationWidth:\n  Width: 4\nLayout/LineLength:\n  Max: 80\nStyle/StringLiterals:\n  Enabled: false\n",
        ),
        (
            "nested/deeper/.rubocop.yml",
            "inherit_from: ../.rubocop.yml\nAllCops:\n  TargetRubyVersion: 3.4\nLayout/IndentationWidth:\n  Width: 6\n",
        ),
    ] {
        std::fs::write(project.path().join(path), yaml).unwrap();
    }
    project
}

#[test]
fn root_and_nearest_layer_configs_match_existing_resolution_and_global_injection() {
    let project = project();
    let config = load_config(None, Some(project.path()), None).unwrap();
    let registry = CopRegistry::default_registry();
    let filters = CopFilterSet::build(&config, &registry);
    let configs = PreparedConfigs::new(&config, &registry);
    let context = prepared(&config, &registry, &filters, &configs);
    for path in ["root.rb", "nested/a.rb", "nested/deeper/a.rb"] {
        compare_configs(context, &project.path().join(path));
    }
    compare_configs(context, Path::new("nested/a.rb"));
}

fn compare_configs(context: PreparedLint<'_>, path: &Path) {
    let prepared = context.for_file(path);
    for name in [
        "Lint/Syntax",
        "Style/StringLiterals",
        "Layout/IndentationWidth",
        "Style/TrailingBodyOnMethodDefinition",
        "Layout/FirstArgumentIndentation",
        "Layout/LineLength",
    ] {
        let index = context.registry.index_of(name).unwrap();
        assert_eq!(
            serde_json::to_value(prepared.get(index)).unwrap(),
            serde_json::to_value(context.config.cop_config_for_file(name, path)).unwrap(),
            "{} {name}",
            path.display()
        );
        assert_eq!(
            prepared.disabled(index),
            context.config.disabled_by_dir_override(name, path)
        );
    }
}

#[test]
fn sibling_files_share_one_effective_layer_without_preparing_unused_cops() {
    let project = project();
    let config = load_config(None, Some(project.path()), None).unwrap();
    let registry = CopRegistry::default_registry();
    let filters = CopFilterSet::build(&config, &registry);
    let configs = PreparedConfigs::new(&config, &registry);
    let context = prepared(&config, &registry, &filters, &configs);
    assert_shared_layer(context, project.path());
    assert!(configs.root.configs.iter().all(|slot| slot.get().is_none()));
}

fn assert_shared_layer(context: PreparedLint<'_>, root: &Path) {
    let first_path = root.join("nested/a.rb");
    let next_path = root.join("nested/b.rb");
    let first = context.for_file(&first_path);
    assert!(first.group.effective.get().is_none());
    assert!(std::ptr::eq(
        first.get(0),
        context.for_file(&next_path).get(0)
    ));
    assert_eq!(
        first
            .group
            .configs
            .iter()
            .filter(|slot| slot.get().is_some())
            .count(),
        1
    );
    assert_eq!(
        context
            .configs
            .layers
            .iter()
            .filter(|slot| slot.get().is_some())
            .count(),
        1
    );
}

#[test]
fn syntax_probe_keeps_root_target_when_registered_syntax_is_not_selected() {
    let project = project();
    let config = load_config(None, Some(project.path()), None).unwrap();
    let registry = CopRegistry::default_registry();
    let filters = CopFilterSet::build(&config, &registry);
    let configs = PreparedConfigs::new(&config, &registry);
    let context = prepared(&config, &registry, &filters, &configs);
    let nested = project.path().join("nested/a.rb");
    let index = registry.index_of("Lint/Syntax").unwrap();
    assert_eq!(context.syntax().get_f64("TargetRubyVersion", 0.0), 3.1);
    assert_eq!(
        context
            .for_file(&nested)
            .get(index)
            .get_f64("TargetRubyVersion", 0.0),
        3.2
    );
}

#[test]
fn independent_preparation_observes_configuration_changes() {
    let mut config = ResolvedConfig::empty();
    let registry = CopRegistry::default_registry();
    let index = registry.index_of("Lint/Syntax").unwrap();
    config.target_ruby_version = Some(3.1);
    assert_target(&config, &registry, index, 3.1);
    config.target_ruby_version = Some(3.4);
    assert_target(&config, &registry, index, 3.4);
}

fn assert_target(config: &ResolvedConfig, registry: &CopRegistry, index: usize, version: f64) {
    let filters = CopFilterSet::build(config, registry);
    let configs = PreparedConfigs::new(config, registry);
    let context = prepared(config, registry, &filters, &configs);
    assert_eq!(
        context
            .for_file(Path::new("test.rb"))
            .get(index)
            .get_f64("TargetRubyVersion", 0.0),
        version
    );
}
