//! Nested `.rubocop.yml` effective config for a file path.

use std::collections::HashSet;
use std::path::Path;

use crate::cop::EnabledState;

use super::ResolvedConfig;
use super::filter_path::path_under_dir;
use super::merge::merge_layer_into;
use super::types::{ConfigLayer, NewCopsPolicy};

impl ResolvedConfig {
    /// True when a nested `.rubocop.yml` disables this cop for `file_path`.
    pub fn disabled_by_dir_override(&self, name: &str, file_path: &Path) -> bool {
        self.dir_layer_index_for_file(file_path)
            .is_some_and(|index| self.dir_override_disables(index, name))
    }

    pub(crate) fn dir_override_disables(&self, index: usize, name: &str) -> bool {
        let layer = &self.dir_overrides[index].1;
        layer
            .cop_configs
            .get(name)
            .is_some_and(|c| c.enabled == EnabledState::False)
            || layer
                .department_configs
                .get(name.split('/').next().unwrap_or(""))
                .is_some_and(|c| c.enabled == EnabledState::False)
    }

    /// Build the effective config for a file under a nested `.rubocop.yml`.
    pub fn effective_config_for_file(&self, file_path: &Path) -> Option<Self> {
        let layer = self.find_dir_layer_for_file(file_path)?;
        let mut effective = self.clone();
        let mut merged = self.layer_snapshot();
        merge_layer_into(&mut merged, layer, Some(&layer.inherit_mode));
        self.apply_merged_layer(&mut effective, &merged);
        self.apply_project_mentions(&mut effective, layer);
        normalize_disabled_by_default(&mut effective);
        Some(effective)
    }

    /// Find the nearest directory-specific config layer, if any.
    fn find_dir_layer_for_file(&self, file_path: &Path) -> Option<&ConfigLayer> {
        self.dir_layer_index_for_file(file_path)
            .map(|index| &self.dir_overrides[index].1)
    }

    pub(crate) fn dir_layer_index_for_file(&self, file_path: &Path) -> Option<usize> {
        if self.dir_overrides.is_empty() {
            return None;
        }
        for (index, (dir, _)) in self.dir_overrides.iter().enumerate() {
            if path_under_dir(file_path, dir) {
                return Some(index);
            }
        }
        self.find_dir_layer_relativized(file_path)
    }

    fn find_dir_layer_relativized(&self, file_path: &Path) -> Option<usize> {
        if let Some(index) =
            self.find_dir_layer_relativized_under(file_path, self.scan_root.as_deref())
        {
            return Some(index);
        }
        if self.scan_root.as_deref() == self.config_dir.as_deref() {
            return None;
        }
        self.find_dir_layer_relativized_under(file_path, self.config_dir.as_deref())
    }

    fn find_dir_layer_relativized_under(
        &self,
        file_path: &Path,
        base: Option<&Path>,
    ) -> Option<usize> {
        let base = base?;
        let rel_path = file_path.strip_prefix(base).unwrap_or(file_path);
        for (index, (dir, _)) in self.dir_overrides.iter().enumerate() {
            let rel_dir = dir.strip_prefix(base).unwrap_or(dir.as_path());
            if rel_path.starts_with(rel_dir) || path_under_dir(rel_path, rel_dir) {
                return Some(index);
            }
        }
        None
    }

    fn layer_snapshot(&self) -> ConfigLayer {
        let mut layer = ConfigLayer::empty();
        self.copy_maps_into(&mut layer);
        self.copy_versions_into(&mut layer);
        self.copy_display_into(&mut layer);
        layer
    }

    fn copy_maps_into(&self, layer: &mut ConfigLayer) {
        layer.cop_configs = self.cop_configs.clone();
        layer.department_configs = self.department_configs.clone();
        layer.global_excludes = self.global_excludes.clone();
        layer.require_known_cops = self.require_known_cops.clone();
        layer.require_departments = self.require_departments.clone();
        layer.user_mentioned_cops = self.project_mentioned_cops.clone();
        layer.user_mentioned_depts = self.project_mentioned_depts.clone();
    }

    fn copy_versions_into(&self, layer: &mut ConfigLayer) {
        layer.new_cops = Some(match self.new_cops {
            NewCopsPolicy::Enable => "enable".to_string(),
            NewCopsPolicy::Disable => "disable".to_string(),
        });
        layer.disabled_by_default = Some(self.disabled_by_default);
        layer.target_ruby_version = self.target_ruby_version;
        layer.target_rails_version = self.target_rails_version;
        layer.active_support_extensions_enabled = Some(self.active_support_extensions_enabled);
        layer.migrated_schema_version = self.migrated_schema_version.clone();
    }

    fn copy_display_into(&self, layer: &mut ConfigLayer) {
        layer.display_cop_names = Some(self.display_cop_names);
        layer.display_style_guide = Some(self.display_style_guide);
        layer.extra_details = Some(self.extra_details);
        layer.style_guide_base_url = self.style_guide_base_url.clone();
    }

    fn apply_merged_layer(&self, effective: &mut Self, merged: &ConfigLayer) {
        effective.cop_configs = merged.cop_configs.clone();
        effective.department_configs = merged.department_configs.clone();
        effective.global_excludes = merged.global_excludes.clone();
        effective.new_cops = match merged.new_cops.as_deref() {
            Some("enable") => NewCopsPolicy::Enable,
            _ => NewCopsPolicy::Disable,
        };
        effective.disabled_by_default = merged
            .disabled_by_default
            .unwrap_or(self.disabled_by_default);
        effective.require_known_cops = merged.require_known_cops.clone();
        effective.require_departments = merged.require_departments.clone();
        effective.target_ruby_version = merged.target_ruby_version;
        effective.target_rails_version = merged.target_rails_version;
        effective.active_support_extensions_enabled = merged
            .active_support_extensions_enabled
            .unwrap_or(self.active_support_extensions_enabled);
        effective.migrated_schema_version = merged.migrated_schema_version.clone();
        self.apply_display_from(effective, merged);
    }

    fn apply_display_from(&self, effective: &mut Self, merged: &ConfigLayer) {
        effective.display_cop_names = merged.display_cop_names.unwrap_or(self.display_cop_names);
        effective.display_style_guide = merged
            .display_style_guide
            .unwrap_or(self.display_style_guide);
        effective.extra_details = merged.extra_details.unwrap_or(self.extra_details);
        if merged.style_guide_base_url.is_some() {
            effective.style_guide_base_url = merged.style_guide_base_url.clone();
        }
    }

    fn apply_project_mentions(&self, effective: &mut Self, layer: &ConfigLayer) {
        effective
            .project_mentioned_cops
            .extend(layer.cop_configs.keys().cloned());
        effective
            .project_mentioned_depts
            .extend(layer.department_configs.keys().cloned());
        for (dept_name, dept_cfg) in &layer.department_configs {
            update_enabled_dept(
                &mut effective.project_enabled_depts,
                dept_name,
                dept_cfg.enabled,
            );
        }
    }
}

fn update_enabled_dept(depts: &mut HashSet<String>, dept_name: &str, enabled: EnabledState) {
    if enabled == EnabledState::True {
        depts.insert(dept_name.to_string());
    } else if enabled != EnabledState::Unset {
        depts.remove(dept_name);
    }
}

fn normalize_disabled_by_default(effective: &mut ResolvedConfig) {
    if !effective.disabled_by_default {
        return;
    }
    for (cop_name, cop_cfg) in &mut effective.cop_configs {
        if cop_cfg.enabled == EnabledState::True
            && !effective.project_mentioned_cops.contains(cop_name)
        {
            cop_cfg.enabled = EnabledState::Unset;
        }
    }
    for (dept_name, dept_cfg) in &mut effective.department_configs {
        if dept_cfg.enabled == EnabledState::True
            && !effective.project_mentioned_depts.contains(dept_name)
        {
            dept_cfg.enabled = EnabledState::Unset;
        }
    }
}
