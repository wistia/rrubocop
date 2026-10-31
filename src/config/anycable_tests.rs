use super::*;
use std::collections::HashMap;
use std::fs;

#[test]
fn anycable_require_loads_opt_in_rules_and_channel_scope() {
    let temp = tempfile::tempdir().unwrap();
    let filters =
        load_anycable(temp.path()).build_cop_filters(&crate::cop::CopRegistry::default_registry());
    for name in [
        "AnyCable/InstanceVars",
        "AnyCable/PeriodicalTimers",
        "AnyCable/StreamFrom",
    ] {
        assert!(
            filters
                .is_cop_enabled_for_file(name, &temp.path().join("app/channels/room_channel.rb"))
        );
        assert!(!filters.is_cop_enabled_for_file(name, &temp.path().join("app/models/room.rb")));
        assert!(
            !filters.is_cop_enabled_for_file(
                name,
                &temp.path().join("app/controllers/channels/room.rb")
            )
        );
    }
}
fn write_gem(path: &std::path::Path) {
    let directory = path.join("lib/anycable/rails/rubocop/config");
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join("default.yml"),"AnyCable/InstanceVars:\n  Enabled: true\n  Include: ['**/channels/**/*.rb']\nAnyCable/PeriodicalTimers:\n  Enabled: true\n  Include: ['**/channels/**/*.rb']\nAnyCable/StreamFrom:\n  Enabled: true\n  Include: ['**/channels/**/*.rb']\n").unwrap();
}

fn load_anycable(path: &std::path::Path) -> ResolvedConfig {
    let gem = path.join("gem");
    write_gem(&gem);
    fs::write(path.join(".rubocop.yml"),"require: anycable/rails/compatibility/rubocop\nAllCops:\n  NewCops: disable\nAnyCable:\n  Exclude: [app/controllers/channels/**/*.rb]\n").unwrap();
    load_config(
        None,
        Some(path),
        Some(&HashMap::from([("anycable-rails-core".into(), gem)])),
    )
    .unwrap()
}

#[test]
fn short_extension_names_enable_real_cops_with_their_options() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join(".rubocop.yml"),"NoSleep:\n  Enabled: true\n  Exclude: ['spec/**/*']\nCops::NoForeignKeys:\n  Enabled: true\n").unwrap();
    let filters = load_config(None, Some(temp.path()), None)
        .unwrap()
        .build_cop_filters(&crate::cop::CopRegistry::default_registry());
    assert!(
        filters.is_cop_enabled_for_file("Cops/NoSleep", &temp.path().join("app/lib/worker.rb"))
    );
    assert!(
        !filters.is_cop_enabled_for_file("Cops/NoSleep", &temp.path().join("spec/worker_spec.rb"))
    );
    assert!(filters.is_cop_enabled_for_file(
        "Cops/NoForeignKeys",
        &temp.path().join("db/migrate/example.rb")
    ));
}

#[test]
fn requiring_a_compiled_extension_enables_its_default_policy() {
    let temp = tempfile::tempdir().unwrap();
    write_extension_fixture(temp.path());
    let config = load_config(None, Some(temp.path()), None).unwrap();
    assert_eq!(
        config
            .cop_config("Cops/NoDirectLanternConfigBuilderMutators")
            .enabled,
        crate::cop::EnabledState::True
    );
    assert!(
        config
            .build_cop_filters(&crate::cop::CopRegistry::default_registry())
            .is_cop_enabled_for_file(
                "Cops/NoDirectLanternConfigBuilderMutators",
                &temp.path().join("app/lib/tools.rb")
            )
    );
}

fn write_extension_fixture(path: &std::path::Path) {
    fs::create_dir(path.join("cops")).unwrap();
    fs::write(
        path.join("cops/lantern.rb"),
        "module Cops; class NoDirectLanternConfigBuilderMutators < RuboCop::Cop::Base; end; end",
    )
    .unwrap();
    fs::write(
        path.join(".rubocop.yml"),
        "require: ./cops/lantern.rb\nAllCops:\n  NewCops: disable\n",
    )
    .unwrap();
}
