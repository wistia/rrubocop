//! Path / version / ResolvedConfig assembly helpers for [`super::load`].

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::cop::EnabledState;

use super::discover::find_config;
use super::load_defaults::try_load_rubocop_defaults;
use super::load_lockfile::LockfileMeta;
use super::load_recursive::{load_config_recursive_inner, load_project_config_recursive};
use super::resolved::ResolvedConfig;
pub(crate) use super::ruby_ver::resolve_target_ruby_version;
use super::standard::convert_standard_yml;
use super::types::{ConfigLayer, NewCopsPolicy};

pub(crate) use super::load_lockfile::resolve_lockfile_meta;

pub(crate) fn resolve_start_dir(target_dir: Option<&Path>) -> Option<PathBuf> {
    let dir = target_dir
        .map(|p| {
            if p.is_file() {
                match p.parent() {
                    Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
                    _ => PathBuf::from("."),
                }
            } else {
                p.to_path_buf()
            }
        })
        .or_else(|| std::env::current_dir().ok())?;
    Some(std::fs::canonicalize(&dir).unwrap_or(dir))
}

pub(crate) fn resolve_config_path(
    path: Option<&Path>,
    start_dir: Option<&PathBuf>,
) -> Option<PathBuf> {
    match path {
        Some(p) if p.exists() => Some(p.to_path_buf()),
        Some(_) => None,
        None => start_dir.and_then(|dir| find_config(dir)),
    }
}

pub(crate) enum ConfigLoadPath {
    Empty,
    NoConfig(PathBuf),
    Resolved {
        config_path: PathBuf,
        scan_root: PathBuf,
    },
}

pub(crate) fn resolve_config_load(
    path: Option<&Path>,
    start_dir: Option<PathBuf>,
) -> ConfigLoadPath {
    let config_path = resolve_config_path(path, start_dir.as_ref());
    if path.is_some() && config_path.is_none() {
        return ConfigLoadPath::Empty;
    }
    let Some(config_path) = config_path else {
        return match start_dir {
            Some(dir) => ConfigLoadPath::NoConfig(dir),
            None => ConfigLoadPath::Empty,
        };
    };
    ConfigLoadPath::Resolved {
        config_path,
        scan_root: start_dir.unwrap_or_else(|| PathBuf::from(".")),
    }
}

pub(crate) fn resolve_path_base_dir(config_path: &Path, config_dir: &Path) -> PathBuf {
    if config_path
        .file_name()
        .and_then(|f| f.to_str())
        .is_some_and(|name| name.starts_with(".rubocop"))
    {
        config_dir
            .canonicalize()
            .unwrap_or_else(|_| config_dir.to_path_buf())
    } else {
        std::env::current_dir().unwrap_or_else(|_| config_dir.to_path_buf())
    }
}

pub(crate) fn load_project_layer(
    config_path: &Path,
    config_dir: &Path,
    gem_cache: Option<&HashMap<String, PathBuf>>,
    override_yaml: Option<&str>,
) -> Result<ConfigLayer> {
    let mut visited = HashSet::new();
    if let Some(yaml) = override_yaml {
        return load_config_recursive_inner(
            config_path,
            config_dir,
            &mut visited,
            gem_cache,
            Some(yaml),
        );
    }
    if config_path
        .file_name()
        .is_some_and(|f| f == ".standard.yml")
    {
        let synthetic_yaml = convert_standard_yml(config_path)?;
        load_config_recursive_inner(
            config_path,
            config_dir,
            &mut visited,
            gem_cache,
            Some(&synthetic_yaml),
        )
    } else {
        load_project_config_recursive(config_path, config_dir, &mut visited, gem_cache)
    }
}

pub(crate) fn project_enabled_depts(project_layer: &ConfigLayer) -> HashSet<String> {
    project_layer
        .department_configs
        .iter()
        .filter(|(name, cfg)| {
            cfg.enabled == EnabledState::True
                && !project_layer.require_enabled_depts.contains(name.as_str())
        })
        .map(|(name, _)| name.clone())
        .collect()
}

pub(crate) fn apply_disabled_by_default(
    base: &mut ConfigLayer,
    project_mentioned_cops: &HashSet<String>,
    project_mentioned_depts: &HashSet<String>,
) {
    if !base.disabled_by_default.unwrap_or(false) {
        return;
    }
    for (cop_name, cop_cfg) in base.cop_configs.iter_mut() {
        if cop_cfg.enabled == EnabledState::True && !project_mentioned_cops.contains(cop_name) {
            cop_cfg.enabled = EnabledState::Unset;
        }
    }
    for (dept_name, dept_cfg) in base.department_configs.iter_mut() {
        if dept_cfg.enabled == EnabledState::True && !project_mentioned_depts.contains(dept_name) {
            dept_cfg.enabled = EnabledState::Unset;
        }
    }
}

pub(crate) struct ResolvedParts {
    pub base: ConfigLayer,
    pub config_dir: PathBuf,
    pub scan_root: Option<PathBuf>,
    pub base_dir: PathBuf,
    pub rubocop_known_cops: HashSet<String>,
    pub project_mentioned_cops: HashSet<String>,
    pub project_mentioned_depts: HashSet<String>,
    pub project_enabled_depts: HashSet<String>,
    pub dir_overrides: Vec<(PathBuf, ConfigLayer)>,
    pub lock: LockfileMeta,
    pub target_ruby_version: Option<f64>,
}

pub(crate) fn build_resolved(parts: ResolvedParts) -> ResolvedConfig {
    let disabled_by_default = parts.base.disabled_by_default.unwrap_or(false);
    let new_cops = match parts.base.new_cops.as_deref() {
        Some("enable") => NewCopsPolicy::Enable,
        _ => NewCopsPolicy::Disable,
    };
    ResolvedConfig {
        cop_configs: parts.base.cop_configs,
        department_configs: parts.base.department_configs,
        global_excludes: parts.base.global_excludes,
        rubocop_version: super::gem_path_version::select_version("rubocop", &parts.config_dir).ok(),
        rubocop_rails_version: super::gem_path_version::select_version(
            "rubocop-rails",
            &parts.config_dir,
        )
        .ok(),
        config_dir: Some(parts.config_dir),
        scan_root: parts.scan_root,
        new_cops,
        disabled_by_default,
        require_known_cops: parts.base.require_known_cops,
        require_departments: parts.base.require_departments,
        target_ruby_version: parts.target_ruby_version.or(Some(2.7)),
        target_rails_version: parts.lock.target_rails_version,
        active_support_extensions_enabled: parts
            .base
            .active_support_extensions_enabled
            .unwrap_or(false),
        rubocop_known_cops: parts.rubocop_known_cops,
        project_mentioned_cops: parts.project_mentioned_cops,
        project_mentioned_depts: parts.project_mentioned_depts,
        project_enabled_depts: parts.project_enabled_depts,
        dir_overrides: parts.dir_overrides,
        railties_in_lockfile: parts.lock.railties_in_lockfile,
        railties_version: parts.lock.railties_version,
        rack_version: parts.lock.rack_version,
        base_dir: Some(parts.base_dir),
        migrated_schema_version: parts.base.migrated_schema_version,
        display_cop_names: parts.base.display_cop_names.unwrap_or(true),
        display_style_guide: parts.base.display_style_guide.unwrap_or(false),
        extra_details: parts.base.extra_details.unwrap_or(false),
        style_guide_base_url: parts.base.style_guide_base_url,
    }
}

/// Built-in RuboCop `default.yml` only (no project layer).
pub(crate) fn defaults_only_resolved(
    config_dir: PathBuf,
    scan_root: Option<PathBuf>,
    gem_cache: Option<&HashMap<String, PathBuf>>,
) -> ResolvedConfig {
    let base_dir = std::env::current_dir().unwrap_or_else(|_| config_dir.clone());
    let (base, rubocop_known_cops) = try_load_rubocop_defaults(&config_dir, gem_cache);
    build_resolved(ResolvedParts {
        target_ruby_version: resolve_target_ruby_version(&base, &config_dir),
        lock: resolve_lockfile_meta(&base, &base_dir),
        dir_overrides: Vec::new(),
        base,
        config_dir,
        scan_root,
        base_dir,
        rubocop_known_cops,
        project_mentioned_cops: HashSet::new(),
        project_mentioned_depts: HashSet::new(),
        project_enabled_depts: HashSet::new(),
    })
}
