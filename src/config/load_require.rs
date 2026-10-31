//! `require:` / `plugins:` gem default config loading.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde_yml::Value;

use crate::cop::EnabledState;

use super::gem_path;
use super::load_recursive::{load_config_recursive, load_config_recursive_inner};
use super::merge::merge_layer_into;
use super::standard::{PLUGIN_GEM_DEPARTMENTS, standard_gem_config_path};
use super::types::ConfigLayer;

fn yaml_string_list(val: &Value) -> Vec<String> {
    match val {
        Value::String(s) => vec![s.clone()],
        Value::Sequence(seq) => seq
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        _ => vec![],
    }
}

pub(crate) fn collect_yaml_strings(val: &Value) -> Vec<String> {
    yaml_string_list(val)
}

fn collect_require_gems(map: &serde_yml::Mapping) -> Vec<String> {
    let mut gems = Vec::new();
    for key in &["plugins", "require"] {
        if let Some(val) = map.get(Value::String(key.to_string())) {
            gems.extend(yaml_string_list(val));
        }
    }
    gems.dedup();
    gems
}

fn needs_wrapper(gems: &[String], injected: &[(usize, String)], name: &str) -> bool {
    !gems.iter().any(|g| g == name) && !injected.iter().any(|(_, g)| g == name)
}

fn inject_one_wrapper(
    gems: &[String],
    injected: &mut Vec<(usize, String)>,
    wrapper: &str,
    rubocop: &str,
) {
    for (i, gem) in gems.iter().enumerate() {
        if gem == wrapper && needs_wrapper(gems, injected, rubocop) {
            injected.push((i, rubocop.to_string()));
        }
    }
}

fn inject_standard_wrappers(gems: &mut Vec<String>) {
    let mut injected = Vec::new();
    inject_one_wrapper(gems, &mut injected, "standard-rails", "rubocop-rails");
    inject_one_wrapper(
        gems,
        &mut injected,
        "standard-performance",
        "rubocop-performance",
    );
    for (i, gem) in injected.into_iter().rev() {
        gems.insert(i, gem);
    }
}

fn gem_config_rel_path(gem_name: &str, ruby_version: Option<f64>) -> Option<String> {
    if gem_name == "anycable-rails-core" {
        Some("lib/anycable/rails/rubocop/config/default.yml".into())
    } else if gem_name.starts_with("rubocop-") {
        Some("config/default.yml".into())
    } else {
        standard_gem_config_path(gem_name, ruby_version).map(|path| path.into())
    }
}

fn load_gem_yaml_layer(
    gem_name: &str,
    rel_path: &str,
    working_dir: &Path,
    visited: &mut HashSet<PathBuf>,
    gem_cache: Option<&HashMap<String, PathBuf>>,
) -> anyhow::Result<ConfigLayer> {
    match gem_path::resolve_gem_config(gem_name, rel_path, working_dir, gem_cache)? {
        gem_path::GemConfigSrc::Disk(root) => {
            load_config_recursive(&root.join(rel_path), working_dir, visited, gem_cache)
        }
        gem_path::GemConfigSrc::Embed { version, yaml } => load_config_recursive_inner(
            &gem_path::virtual_config_path(gem_name, &version, rel_path),
            working_dir,
            visited,
            gem_cache,
            Some(yaml),
        ),
    }
}

fn merge_require_layer(base_layer: &mut ConfigLayer, mut layer: ConfigLayer, working_dir: &Path) {
    // Legacy require layers do not carry their merge policy into the project.
    // New plugin defaults retain injected excludes after project replacement.
    if !super::gem_path_version::select_version("rubocop", working_dir)
        .is_ok_and(|v| v.starts_with("1.65."))
    {
        base_layer.extend_plugin_excludes(&layer.global_excludes);
    } else {
        layer.inherit_mode = Default::default();
    }
    merge_layer_into(base_layer, &layer, None);
}

fn load_require_fallback(
    gem_name: &str,
    working_dir: &Path,
    visited: &mut HashSet<PathBuf>,
    gem_cache: Option<&HashMap<String, PathBuf>>,
    base_layer: &mut ConfigLayer,
) {
    if gem_name.starts_with("rubocop-") {
        return;
    }
    match load_gem_yaml_layer(gem_name, "config/base.yml", working_dir, visited, gem_cache) {
        Ok(layer) => merge_layer_into(base_layer, &layer, None),
        Err(_) => {}
    }
}

fn load_one_require_gem(
    gem_name: &str,
    ruby_version: Option<f64>,
    working_dir: &Path,
    visited: &mut HashSet<PathBuf>,
    gem_cache: Option<&HashMap<String, PathBuf>>,
    base_layer: &mut ConfigLayer,
) {
    let gem_name = if gem_name == "anycable/rails/compatibility/rubocop" {
        "anycable-rails-core"
    } else {
        gem_name
    };
    let Some(config_rel_path) = gem_config_rel_path(gem_name, ruby_version) else {
        return;
    };
    match load_gem_yaml_layer(gem_name, &config_rel_path, working_dir, visited, gem_cache) {
        Ok(layer) => merge_require_layer(base_layer, layer, working_dir),
        Err(e) => {
            if gem_name.starts_with("rubocop-") {
                eprintln!("warning: require '{gem_name}': {e:#}");
                return;
            }
            load_require_fallback(gem_name, working_dir, visited, gem_cache, base_layer);
        }
    }
}

fn record_require_enabled(base_layer: &mut ConfigLayer) {
    base_layer.require_enabled_cops = base_layer
        .cop_configs
        .iter()
        .filter(|(_, c)| c.enabled == EnabledState::True)
        .map(|(n, _)| n.clone())
        .collect();
    base_layer.require_enabled_depts = base_layer
        .department_configs
        .iter()
        .filter(|(_, c)| c.enabled == EnabledState::True)
        .map(|(n, _)| n.clone())
        .collect();
}

fn register_plugin_depts(base_layer: &mut ConfigLayer, gems: &[String]) {
    base_layer.require_known_cops = base_layer.cop_configs.keys().cloned().collect();
    base_layer.require_departments = base_layer.department_configs.keys().cloned().collect();
    for gem_name in gems {
        for (dept, gem) in PLUGIN_GEM_DEPARTMENTS {
            if gem_name.as_str() == *gem {
                base_layer.require_departments.insert(dept.to_string());
            }
        }
    }
}

/// Process `plugins:` / `require:` gem defaults into `base_layer`.
pub(crate) fn process_require_plugins(
    map: &serde_yml::Mapping,
    ruby_version: Option<f64>,
    working_dir: &Path,
    visited: &mut HashSet<PathBuf>,
    gem_cache: Option<&HashMap<String, PathBuf>>,
    base_layer: &mut ConfigLayer,
) {
    let mut gems = collect_require_gems(map);
    inject_standard_wrappers(&mut gems);
    let visited_before = visited.clone();
    for gem_name in &gems {
        super::load_extension::load_required(gem_name, working_dir, base_layer);
        load_one_require_gem(
            gem_name,
            ruby_version,
            working_dir,
            visited,
            gem_cache,
            base_layer,
        );
    }
    record_require_enabled(base_layer);
    *visited = visited_before;
    register_plugin_depts(base_layer, &gems);
}
