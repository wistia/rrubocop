//! Top-level config loading and inheritance resolution.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;

use super::discover::load_dir_overrides;
use super::load_defaults::try_load_rubocop_defaults;
use super::load_resolve::{
    ConfigLoadPath, ResolvedParts, apply_disabled_by_default, build_resolved,
    defaults_only_resolved, load_project_layer, project_enabled_depts, resolve_config_load,
    resolve_lockfile_meta, resolve_path_base_dir, resolve_start_dir, resolve_target_ruby_version,
};
use super::merge::merge_layer_into;
use super::rails_app::{OMAKASE_DEFAULT_YAML, is_rails_app};
use super::resolved::ResolvedConfig;

fn merge_project_onto_defaults(
    config_path: &Path,
    config_dir: &Path,
    gem_cache: Option<&HashMap<String, PathBuf>>,
    override_yaml: Option<&str>,
    base: &mut super::types::ConfigLayer,
) -> Result<(
    std::collections::HashSet<String>,
    std::collections::HashSet<String>,
    std::collections::HashSet<String>,
)> {
    let project_layer = load_project_layer(config_path, config_dir, gem_cache, override_yaml)?;
    let cops = project_layer.user_mentioned_cops.clone();
    let depts = project_layer.user_mentioned_depts.clone();
    let enabled = project_enabled_depts(&project_layer);
    merge_layer_into(base, &project_layer, Some(&project_layer.inherit_mode));
    // Plugin excludes survive project replace; also keep them when merging onto defaults.
    base.extend_plugin_excludes(&project_layer.plugin_excludes);
    base.reapply_plugin_excludes();
    apply_disabled_by_default(base, &cops, &depts);
    Ok((cops, depts, enabled))
}

fn config_parent(config_path: &Path) -> PathBuf {
    match config_path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

fn dir_overrides_for(
    explicit: bool,
    scan_root: &Path,
) -> Vec<(PathBuf, super::types::ConfigLayer)> {
    if explicit {
        Vec::new()
    } else {
        load_dir_overrides(scan_root)
    }
}

fn assemble_resolved(
    explicit_config_path: bool,
    config_path: PathBuf,
    scan_root: PathBuf,
    gem_cache: Option<&HashMap<String, PathBuf>>,
    override_yaml: Option<&str>,
) -> Result<ResolvedConfig> {
    let config_dir = config_parent(&config_path);
    let base_dir = resolve_path_base_dir(&config_path, &config_dir);
    let (mut base, rubocop_known_cops) = try_load_rubocop_defaults(&config_dir, gem_cache);
    let (project_mentioned_cops, project_mentioned_depts, project_enabled_depts) =
        merge_project_onto_defaults(
            &config_path,
            &config_dir,
            gem_cache,
            override_yaml,
            &mut base,
        )?;
    Ok(build_resolved(ResolvedParts {
        target_ruby_version: resolve_target_ruby_version(&base, &config_dir),
        lock: resolve_lockfile_meta(&base, &base_dir),
        dir_overrides: dir_overrides_for(explicit_config_path, &scan_root),
        base,
        config_dir,
        scan_root: Some(scan_root),
        base_dir,
        rubocop_known_cops,
        project_mentioned_cops,
        project_mentioned_depts,
        project_enabled_depts,
    }))
}

pub fn load_config(
    path: Option<&Path>,
    target_dir: Option<&Path>,
    gem_cache: Option<&HashMap<String, PathBuf>>,
) -> Result<ResolvedConfig> {
    super::lockfile_snapshot::with_snapshot(|| {
        match resolve_config_load(path, resolve_start_dir(target_dir)) {
            ConfigLoadPath::Empty => Ok(ResolvedConfig::empty()),
            ConfigLoadPath::NoConfig(dir) => no_config_resolved(dir, gem_cache),
            ConfigLoadPath::Resolved {
                config_path,
                scan_root,
            } => assemble_resolved(path.is_some(), config_path, scan_root, gem_cache, None),
        }
    })
}

/// No `.rubocop.yml`/`.standard.yml` found: Rails apps (`rails new`, 7.2+)
/// default to `rubocop-rails-omakase` via `inherit_gem` with no config file
/// on disk, so synthesize that inheritance; otherwise fall back to RuboCop's
/// own built-in defaults.
fn no_config_resolved(
    dir: PathBuf,
    gem_cache: Option<&HashMap<String, PathBuf>>,
) -> Result<ResolvedConfig> {
    if is_rails_app(&dir) {
        let virtual_config = dir.join(".rubocop.yml");
        return assemble_resolved(
            false,
            virtual_config,
            dir,
            gem_cache,
            Some(OMAKASE_DEFAULT_YAML),
        );
    }
    Ok(empty_resolved_no_config(dir))
}

fn empty_resolved_no_config(config_dir: PathBuf) -> ResolvedConfig {
    // No project `.rubocop.yml`: still apply RuboCop's built-in default.yml.
    let mut cfg = defaults_only_resolved(config_dir.clone(), Some(config_dir.clone()), None);
    cfg.dir_overrides = load_dir_overrides(&config_dir);
    cfg
}

/// RuboCop built-in defaults only (`--force-default-config`).
///
/// Loads vendored `config/default.yml` so `Enabled: false` cops stay off —
/// matching RuboCop, not an empty config that enables every registered cop.
pub fn load_default_config(
    target_dir: Option<&Path>,
    gem_cache: Option<&HashMap<String, PathBuf>>,
) -> ResolvedConfig {
    super::lockfile_snapshot::with_snapshot(|| {
        defaults_only_resolved(
            resolve_start_dir(target_dir).unwrap_or_else(|| PathBuf::from(".")),
            None,
            gem_cache,
        )
    })
}
