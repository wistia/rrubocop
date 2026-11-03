use super::*;
use std::fs;

#[test]
fn plugin_classes_without_yaml_defaults_preserve_version_and_user_controls() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("Gemfile.lock"),
        "GEM\n  specs:\n    rubocop-graphql (1.5.4)\n",
    )
    .unwrap();
    let config_path = temp.path().join(".rubocop.yml");
    fs::write(&config_path, "require: rubocop-graphql\n").unwrap();
    let config = load_config(None, Some(temp.path()), None).unwrap();
    let file = temp.path().join("app/graphql/types/query.rb");
    assert!(
        config
            .enabled_cop_names()
            .contains(&"GraphQL/PrepareMethod".to_string())
    );
    assert!(config.is_cop_enabled("GraphQL/PrepareMethod", &file, &[], &[]));
    fs::write(
        &config_path,
        "require: rubocop-graphql\nGraphQL/PrepareMethod:\n  Enabled: false\n",
    )
    .unwrap();
    let config = load_config(None, Some(temp.path()), None).unwrap();
    assert!(!config.is_cop_enabled("GraphQL/PrepareMethod", &file, &[], &[]));
    fs::write(&config_path, "AllCops:\n  NewCops: disable\n").unwrap();
    let config = load_config(None, Some(temp.path()), None).unwrap();
    assert!(!config.is_cop_enabled("GraphQL/PrepareMethod", &file, &[], &[]));
    assert!(super::gem_configs::registered_without_defaults("rubocop-graphql", "1.8.0").is_empty());
}
