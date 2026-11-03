//! YAML → ConfigLayer parsing.

use serde_yml::Value;

use super::parse_allcops::parse_allcops;
use super::parse_cop::{parse_cop_config, parse_enabled_state};
use super::types::{ConfigLayer, DepartmentConfig, InheritMode};

pub(crate) fn parse_config_layer(raw: &Value) -> ConfigLayer {
    let mut layer = ConfigLayer::empty();
    if let Value::Mapping(map) = raw {
        for (key, value) in map {
            if let Some(key_str) = key.as_str() {
                if let Some(settings) = super::safety::capture(key_str, value) {
                    layer.validation_safety.push(settings);
                }
                apply_layer_entry(&mut layer, key_str, value);
            }
        }
    }
    layer
}

fn apply_layer_entry(layer: &mut ConfigLayer, key_str: &str, value: &Value) {
    match key_str {
        "inherit_from" | "inherit_gem" | "require" | "plugins" => {}
        "inherit_mode" => {
            layer.inherit_mode = parse_inherit_mode(value);
        }
        "AllCops" => {
            apply_allcops_fields(layer, value);
        }
        _ => apply_policy_entry(layer, key_str, value),
    }
}

fn apply_allcops_fields(layer: &mut ConfigLayer, value: &Value) {
    let fields = parse_allcops(value);
    layer.global_excludes = fields.global_excludes;
    layer.new_cops = fields.new_cops;
    layer.disabled_by_default = fields.disabled_by_default;
    layer.target_ruby_version = fields.target_ruby_version;
    layer.target_rails_version = fields.target_rails_version;
    layer.active_support_extensions_enabled = fields.active_support_extensions_enabled;
    layer.migrated_schema_version = fields.migrated_schema_version;
    layer.display_cop_names = fields.display_cop_names;
    layer.display_style_guide = fields.display_style_guide;
    layer.extra_details = fields.extra_details;
    layer.style_guide_base_url = fields.style_guide_base_url;
}

/// Parse a department-level config (e.g. `RSpec:` or `Rails:`).
pub(crate) fn parse_department_config(value: &Value) -> DepartmentConfig {
    let mut config = DepartmentConfig::default();
    if let Value::Mapping(map) = value {
        for (k, v) in map {
            apply_dept_key(&mut config, k.as_str(), v);
        }
    }
    config
}

fn apply_dept_key(config: &mut DepartmentConfig, key: Option<&str>, v: &Value) {
    match key {
        Some("Enabled") => {
            if let Some(state) = parse_enabled_state(v) {
                config.enabled = state;
            }
        }
        Some("Include") => {
            if let Some(list) = value_to_string_list(v) {
                config.include = list;
            }
        }
        Some("Exclude") => {
            if let Some(list) = value_to_string_list(v) {
                config.exclude = list;
            }
        }
        _ => {}
    }
}

/// Parse the `inherit_mode` key from a config file.
pub(crate) fn parse_inherit_mode(value: &Value) -> InheritMode {
    let mut mode = InheritMode::default();
    if let Value::Mapping(map) = value {
        apply_inherit_merge(&mut mode, map);
        apply_inherit_override(&mut mode, map);
    }
    mode
}

fn apply_inherit_merge(mode: &mut InheritMode, map: &serde_yml::Mapping) {
    if let Some(merge_value) = map.get(Value::String("merge".to_string())) {
        if let Some(list) = value_to_string_list(merge_value) {
            mode.merge = list.into_iter().collect();
        }
    }
}

fn apply_inherit_override(mode: &mut InheritMode, map: &serde_yml::Mapping) {
    if let Some(override_value) = map.get(Value::String("override".to_string())) {
        if let Some(list) = value_to_string_list(override_value) {
            mode.override_keys = list.into_iter().collect();
        }
    }
}

pub(crate) fn extract_string_list(value: &Value, key: &str) -> Option<Vec<String>> {
    value
        .as_mapping()?
        .get(Value::String(key.to_string()))?
        .as_sequence()
        .map(|seq| {
            seq.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
}

pub(crate) fn value_to_string_list(value: &Value) -> Option<Vec<String>> {
    value.as_sequence().map(|seq| {
        seq.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()
    })
}

fn apply_policy_entry(layer: &mut ConfigLayer, name: &str, value: &Value) {
    if let Some(qualified) = super::cop_names::qualified(name) {
        layer
            .cop_configs
            .insert(qualified.into_owned(), parse_cop_config(value));
    } else {
        layer
            .department_configs
            .insert(name.into(), parse_department_config(value));
    }
}
