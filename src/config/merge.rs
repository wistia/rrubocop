//! Layer / department config merging.

use crate::cop::EnabledState;

use super::merge_cop::merge_cop_config;
use super::types::{ConfigLayer, DepartmentConfig, InheritMode};

pub(crate) fn merge_layer_into(
    base: &mut ConfigLayer,
    overlay: &ConfigLayer,
    inherit_mode: Option<&InheritMode>,
) {
    super::safety::merge(&mut base.validation_safety, &overlay.validation_safety);
    merge_global_excludes(base, overlay, inherit_mode);
    merge_layer_scalars(base, overlay);
    merge_department_map(base, overlay, inherit_mode);
    merge_cop_map(base, overlay, inherit_mode);
    merge_require_sets(base, overlay);
}

fn merge_global_excludes(
    base: &mut ConfigLayer,
    overlay: &ConfigLayer,
    inherit_mode: Option<&InheritMode>,
) {
    // RuboCop `should_union?`: arrays replace by default. Union only when
    // inherit_mode.merge includes the key (child or already-on-base parent).
    if overlay.global_excludes.is_empty() {
        return;
    }
    let should_replace = !exclude_should_union(base, overlay, inherit_mode);
    merge_exclude_list(
        &mut base.global_excludes,
        &overlay.global_excludes,
        should_replace,
    );
}

/// RuboCop ConfigLoaderResolver#should_union? for AllCops.Exclude.
fn exclude_should_union(
    base: &ConfigLayer,
    overlay: &ConfigLayer,
    inherit_mode: Option<&InheritMode>,
) -> bool {
    let child = inherit_mode.unwrap_or(&overlay.inherit_mode);
    if child.override_keys.contains("Exclude") {
        return false;
    }
    if child.merge.contains("Exclude") {
        return true;
    }
    if base.inherit_mode.override_keys.contains("Exclude") {
        return false;
    }
    base.inherit_mode.merge.contains("Exclude")
}

fn merge_layer_scalars(base: &mut ConfigLayer, overlay: &ConfigLayer) {
    merge_inherit_mode(&mut base.inherit_mode, &overlay.inherit_mode);
    overlay_clone(&mut base.new_cops, &overlay.new_cops);
    overlay_copy(&mut base.disabled_by_default, overlay.disabled_by_default);
    overlay_copy(&mut base.target_ruby_version, overlay.target_ruby_version);
    overlay_copy(&mut base.target_rails_version, overlay.target_rails_version);
    overlay_copy(
        &mut base.active_support_extensions_enabled,
        overlay.active_support_extensions_enabled,
    );
    overlay_clone(
        &mut base.migrated_schema_version,
        &overlay.migrated_schema_version,
    );
    overlay_copy(&mut base.display_cop_names, overlay.display_cop_names);
    overlay_copy(&mut base.display_style_guide, overlay.display_style_guide);
    overlay_copy(&mut base.extra_details, overlay.extra_details);
    overlay_clone(
        &mut base.style_guide_base_url,
        &overlay.style_guide_base_url,
    );
}

fn merge_inherit_mode(base: &mut InheritMode, overlay: &InheritMode) {
    base.merge.extend(overlay.merge.iter().cloned());
    base.override_keys
        .extend(overlay.override_keys.iter().cloned());
}

fn overlay_clone<T: Clone>(base: &mut Option<T>, overlay: &Option<T>) {
    if overlay.is_some() {
        base.clone_from(overlay);
    }
}

fn overlay_copy<T: Copy>(base: &mut Option<T>, overlay: Option<T>) {
    if overlay.is_some() {
        *base = overlay;
    }
}

fn merge_department_map(
    base: &mut ConfigLayer,
    overlay: &ConfigLayer,
    inherit_mode: Option<&InheritMode>,
) {
    for (dept_name, overlay_dept) in &overlay.department_configs {
        match base.department_configs.get_mut(dept_name) {
            Some(base_dept) => {
                merge_department_config(base_dept, overlay_dept, inherit_mode);
            }
            None => {
                base.department_configs
                    .insert(dept_name.clone(), overlay_dept.clone());
            }
        }
    }
}

fn merge_cop_map(
    base: &mut ConfigLayer,
    overlay: &ConfigLayer,
    inherit_mode: Option<&InheritMode>,
) {
    for (cop_name, overlay_config) in &overlay.cop_configs {
        match base.cop_configs.get_mut(cop_name) {
            Some(base_config) => {
                merge_cop_config(base_config, overlay_config, inherit_mode);
            }
            None => {
                base.cop_configs
                    .insert(cop_name.clone(), overlay_config.clone());
            }
        }
        // Track require-originated enabled state through merges.
        if overlay.require_enabled_cops.contains(cop_name) {
            base.require_enabled_cops.insert(cop_name.clone());
        } else if overlay_config.enabled != EnabledState::Unset {
            base.require_enabled_cops.remove(cop_name);
        }
    }
}

fn merge_require_sets(base: &mut ConfigLayer, overlay: &ConfigLayer) {
    for (dept_name, overlay_dept) in &overlay.department_configs {
        if overlay.require_enabled_depts.contains(dept_name) {
            base.require_enabled_depts.insert(dept_name.clone());
        } else if overlay_dept.enabled != EnabledState::Unset {
            base.require_enabled_depts.remove(dept_name);
        }
    }
    // Propagate require-known cops and departments (union — once known, always known)
    for cop in &overlay.require_known_cops {
        base.require_known_cops.insert(cop.clone());
    }
    for dept in &overlay.require_departments {
        base.require_departments.insert(dept.clone());
    }
}

/// Merge a single department's overlay config into its base config.
pub(crate) fn merge_department_config(
    base: &mut DepartmentConfig,
    overlay: &DepartmentConfig,
    inherit_mode: Option<&InheritMode>,
) {
    if overlay.enabled != EnabledState::Unset {
        base.enabled = overlay.enabled;
    }
    let should_merge_include = inherit_mode
        .map(|im| im.merge.contains("Include"))
        .unwrap_or(false);
    merge_exclude_list(
        &mut base.exclude,
        &overlay.exclude,
        inherit_mode
            .map(|im| im.override_keys.contains("Exclude"))
            .unwrap_or(false),
    );
    merge_include_list(&mut base.include, &overlay.include, should_merge_include);
}

pub(crate) fn merge_exclude_list(
    base: &mut Vec<String>,
    overlay: &[String],
    should_override: bool,
) {
    if should_override {
        if !overlay.is_empty() {
            *base = overlay.to_vec();
        }
    } else {
        for exc in overlay {
            if !base.contains(exc) {
                base.push(exc.clone());
            }
        }
    }
}

pub(crate) fn merge_include_list(base: &mut Vec<String>, overlay: &[String], should_merge: bool) {
    if overlay.is_empty() {
        return;
    }
    if should_merge {
        for inc in overlay {
            if !base.contains(inc) {
                base.push(inc.clone());
            }
        }
    } else {
        *base = overlay.to_vec();
    }
}
