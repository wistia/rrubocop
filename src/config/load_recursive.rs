//! Recursive config file loading and inheritance.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_yml::Value;

use super::load_gems::process_inherit_gem;
use super::load_require::process_require_plugins;
use super::merge::merge_layer_into;
use super::parse::parse_config_layer;
use super::types::ConfigLayer;

fn abs_config_path(config_path: &Path) -> PathBuf {
    if config_path.is_absolute() {
        config_path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_default()
            .join(config_path)
    }
}

fn read_config_contents(
    config_path: &Path,
    working_dir: &Path,
    override_contents: Option<&str>,
) -> Result<String> {
    if let Some(s) = override_contents {
        return Ok(s.replace("!ruby/regexp ", ""));
    }
    super::yaml_read::load_yaml_text(config_path, working_dir)
}

fn parse_config_yaml(config_path: &Path, contents: &str) -> Result<Value> {
    serde_yml::from_str(contents)
        .with_context(|| format!("failed to parse {}", config_path.display()))
}

fn peek_local_ruby_version(map: &serde_yml::Mapping) -> Option<f64> {
    let ac = map.get(Value::String("AllCops".to_string()))?;
    let Value::Mapping(ac_map) = ac else {
        return None;
    };
    let v = ac_map.get(Value::String("TargetRubyVersion".to_string()))?;
    v.as_f64().or_else(|| v.as_u64().map(|u| u as f64))
}

fn inherit_from_paths(inherit_value: &Value) -> Vec<String> {
    match inherit_value {
        Value::String(s) => vec![s.clone()],
        Value::Sequence(seq) => seq
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        _ => vec![],
    }
}

fn merge_inherited_layer(base_layer: &mut ConfigLayer, layer: ConfigLayer) {
    base_layer
        .user_mentioned_cops
        .extend(layer.user_mentioned_cops.iter().cloned());
    base_layer
        .user_mentioned_depts
        .extend(layer.user_mentioned_depts.iter().cloned());
    base_layer.extend_plugin_excludes(&layer.plugin_excludes);
    merge_layer_into(base_layer, &layer, None);
}

fn load_one_inherit_from(
    config_path: &Path,
    config_dir: &Path,
    rel_path: &str,
    working_dir: &Path,
    visited: &mut HashSet<PathBuf>,
    gem_cache: Option<&HashMap<String, PathBuf>>,
    base_layer: &mut ConfigLayer,
) -> Result<()> {
    let inherited_path = config_dir.join(rel_path);
    if !inherited_path.exists() {
        eprintln!(
            "warning: inherit_from target not found: {} (from {})",
            inherited_path.display(),
            config_path.display()
        );
        return Ok(());
    }
    let layer = load_config_recursive(&inherited_path, working_dir, visited, gem_cache)
        .with_context(|| {
            format!(
                "failed to load inherited config {}",
                inherited_path.display()
            )
        })?;
    merge_inherited_layer(base_layer, layer);
    Ok(())
}

fn process_inherit_from(
    map: &serde_yml::Mapping,
    config_path: &Path,
    config_dir: &Path,
    working_dir: &Path,
    visited: &mut HashSet<PathBuf>,
    gem_cache: Option<&HashMap<String, PathBuf>>,
    base_layer: &mut ConfigLayer,
) -> Result<()> {
    let Some(inherit_value) = map.get(Value::String("inherit_from".to_string())) else {
        return Ok(());
    };
    for rel_path in &inherit_from_paths(inherit_value) {
        load_one_inherit_from(
            config_path,
            config_dir,
            rel_path,
            working_dir,
            visited,
            gem_cache,
            base_layer,
        )?;
    }
    Ok(())
}

fn merge_local_layer(base_layer: &mut ConfigLayer, raw: &Value) {
    let local_layer = parse_config_layer(raw);
    base_layer
        .user_mentioned_cops
        .extend(local_layer.cop_configs.keys().cloned());
    base_layer
        .user_mentioned_depts
        .extend(local_layer.department_configs.keys().cloned());
    merge_layer_into(base_layer, &local_layer, Some(&local_layer.inherit_mode));
    // RuboCop plugins write Exclude into default_configuration; project
    // Exclude replace never removes them.
    base_layer.reapply_plugin_excludes();
}

fn process_inheritance_map(
    map: &serde_yml::Mapping,
    config_path: &Path,
    config_dir: &Path,
    working_dir: &Path,
    visited: &mut HashSet<PathBuf>,
    gem_cache: Option<&HashMap<String, PathBuf>>,
    base_layer: &mut ConfigLayer,
) -> Result<()> {
    process_require_plugins(
        map,
        peek_local_ruby_version(map),
        working_dir,
        visited,
        gem_cache,
        base_layer,
    );
    process_inherit_gem(map, working_dir, visited, gem_cache, base_layer)?;
    process_inherit_from(
        map,
        config_path,
        config_dir,
        working_dir,
        visited,
        gem_cache,
        base_layer,
    )?;
    Ok(())
}

/// Recursively load a config file and all its inherited configs.
pub(crate) fn load_config_recursive(
    config_path: &Path,
    working_dir: &Path,
    visited: &mut HashSet<PathBuf>,
    gem_cache: Option<&HashMap<String, PathBuf>>,
) -> Result<ConfigLayer> {
    // Inherited parents export only their own inherit_mode.
    load_config_with_mode(config_path, working_dir, visited, gem_cache, None, true)
}

/// Load the project root config; keep accumulated inherit_mode for defaults merge.
pub(crate) fn load_project_config_recursive(
    config_path: &Path,
    working_dir: &Path,
    visited: &mut HashSet<PathBuf>,
    gem_cache: Option<&HashMap<String, PathBuf>>,
) -> Result<ConfigLayer> {
    load_config_with_mode(config_path, working_dir, visited, gem_cache, None, false)
}

fn config_parent_dir(config_path: &Path) -> PathBuf {
    match config_path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

fn build_layer_from_raw(
    config_path: &Path,
    working_dir: &Path,
    visited: &mut HashSet<PathBuf>,
    gem_cache: Option<&HashMap<String, PathBuf>>,
    raw: &Value,
    export_own_inherit_mode: bool,
) -> Result<ConfigLayer> {
    // When this layer is an inherit_gem/from parent, only export inherit_mode
    // declared in this file (RuboCop key-order: nested modes don't apply to
    // how a child's Exclude replaces this file's Exclude). Root project keeps
    // accumulated mode for merge_with_default.
    let own_mode = parse_config_layer(raw).inherit_mode;
    let config_dir = config_parent_dir(config_path);
    let mut base_layer = ConfigLayer::empty();
    if let Value::Mapping(map) = raw {
        process_inheritance_map(
            map,
            config_path,
            &config_dir,
            working_dir,
            visited,
            gem_cache,
            &mut base_layer,
        )?;
    }
    merge_local_layer(&mut base_layer, raw);
    super::safety::validate(&base_layer.validation_safety, config_path)?;
    if export_own_inherit_mode {
        base_layer.inherit_mode = own_mode;
    }
    Ok(base_layer)
}

/// Like [`load_config_recursive`], with optional synthetic YAML contents.
pub(crate) fn load_config_recursive_inner(
    config_path: &Path,
    working_dir: &Path,
    visited: &mut HashSet<PathBuf>,
    gem_cache: Option<&HashMap<String, PathBuf>>,
    override_contents: Option<&str>,
) -> Result<ConfigLayer> {
    load_config_with_mode(
        config_path,
        working_dir,
        visited,
        gem_cache,
        override_contents,
        true,
    )
}

fn load_config_with_mode(
    config_path: &Path,
    working_dir: &Path,
    visited: &mut HashSet<PathBuf>,
    gem_cache: Option<&HashMap<String, PathBuf>>,
    override_contents: Option<&str>,
    export_own_inherit_mode: bool,
) -> Result<ConfigLayer> {
    if !visited.insert(abs_config_path(config_path)) {
        return Ok(ConfigLayer::empty());
    }
    let contents = read_config_contents(config_path, working_dir, override_contents)?;
    let raw = parse_config_yaml(config_path, &contents)?;
    build_layer_from_raw(
        config_path,
        working_dir,
        visited,
        gem_cache,
        &raw,
        export_own_inherit_mode,
    )
}
