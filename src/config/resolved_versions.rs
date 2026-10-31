use super::ResolvedConfig;
use super::resolved_inject::{inject_bool, inject_f64};
use crate::cop::CopConfig;
use serde_yml::Value;

pub(crate) fn inject_globals(cfg: &ResolvedConfig, config: &mut CopConfig) {
    if let Some(version) = &cfg.rubocop_version {
        config
            .options
            .insert("__RubocopVersion".into(), Value::String(version.clone()));
    }
    if let Some(version) = &cfg.rubocop_rails_version {
        config.options.insert(
            "__RubocopRailsVersion".into(),
            Value::String(version.clone()),
        );
    }
    inject_f64(
        &mut config.options,
        "TargetRubyVersion",
        cfg.target_ruby_version,
    );
    inject_f64(
        &mut config.options,
        "TargetRailsVersion",
        cfg.target_rails_version,
    );
    inject_bool(
        &mut config.options,
        "__RailtiesInLockfile",
        cfg.railties_in_lockfile,
    );
    inject_f64(
        &mut config.options,
        "__RailtiesVersion",
        cfg.railties_version,
    );
}
