//! Config types shared across the config module.

use std::collections::{HashMap, HashSet};

use crate::cop::{CopConfig, EnabledState};

/// Policy for handling `Enabled: pending` cops, controlled by `AllCops.NewCops`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewCopsPolicy {
    Enable,
    Disable,
}

/// Department-level configuration (e.g., `RSpec:`, `Rails:`).
#[derive(Debug, Clone, Default)]
pub(crate) struct DepartmentConfig {
    pub(crate) enabled: EnabledState,
    pub(crate) include: Vec<String>,
    pub(crate) exclude: Vec<String>,
}

/// Controls how arrays are merged during config inheritance.
#[derive(Debug, Clone, Default)]
pub(crate) struct InheritMode {
    pub(crate) merge: HashSet<String>,
    pub(crate) override_keys: HashSet<String>,
}

/// Raw flags preserve explicit nulls and section order before defaults merge.
#[derive(Debug, Clone)]
pub(crate) struct SafetySettings {
    pub(crate) section: String,
    pub(crate) safe: Option<serde_yml::Value>,
    pub(crate) safe_autocorrect: Option<serde_yml::Value>,
}

/// A single parsed config layer (before merging).
#[derive(Debug, Clone)]
pub(crate) struct ConfigLayer {
    /// User/inherited settings; required gem defaults do not propagate here.
    pub(crate) validation_safety: Vec<SafetySettings>,
    pub(crate) cop_configs: HashMap<String, CopConfig>,
    pub(crate) department_configs: HashMap<String, DepartmentConfig>,
    pub(crate) global_excludes: Vec<String>,
    /// AllCops.Exclude from `plugins`/`require` gems — RuboCop injects these
    /// into default_configuration so project Exclude replace cannot drop them.
    pub(crate) plugin_excludes: Vec<String>,
    pub(crate) new_cops: Option<String>,
    pub(crate) disabled_by_default: Option<bool>,
    pub(crate) inherit_mode: InheritMode,
    pub(crate) require_enabled_cops: HashSet<String>,
    pub(crate) require_enabled_depts: HashSet<String>,
    pub(crate) require_known_cops: HashSet<String>,
    pub(crate) require_departments: HashSet<String>,
    pub(crate) user_mentioned_cops: HashSet<String>,
    pub(crate) user_mentioned_depts: HashSet<String>,
    pub(crate) target_ruby_version: Option<f64>,
    pub(crate) target_rails_version: Option<f64>,
    pub(crate) active_support_extensions_enabled: Option<bool>,
    pub(crate) migrated_schema_version: Option<String>,
    pub(crate) display_cop_names: Option<bool>,
    pub(crate) display_style_guide: Option<bool>,
    pub(crate) extra_details: Option<bool>,
    pub(crate) style_guide_base_url: Option<String>,
}

impl ConfigLayer {
    pub(crate) fn empty() -> Self {
        Self {
            validation_safety: Vec::new(),
            cop_configs: HashMap::new(),
            department_configs: HashMap::new(),
            global_excludes: Vec::new(),
            plugin_excludes: Vec::new(),
            new_cops: None,
            disabled_by_default: None,
            inherit_mode: InheritMode::default(),
            require_enabled_cops: HashSet::new(),
            require_enabled_depts: HashSet::new(),
            require_known_cops: HashSet::new(),
            user_mentioned_cops: HashSet::new(),
            user_mentioned_depts: HashSet::new(),
            require_departments: HashSet::new(),
            target_ruby_version: None,
            target_rails_version: None,
            active_support_extensions_enabled: None,
            migrated_schema_version: None,
            display_cop_names: None,
            display_style_guide: None,
            extra_details: None,
            style_guide_base_url: None,
        }
    }

    /// Record plugin AllCops.Exclude patterns (survive later Exclude replace).
    pub(crate) fn extend_plugin_excludes(&mut self, excludes: &[String]) {
        for ex in excludes {
            push_unique(&mut self.plugin_excludes, ex);
        }
    }

    /// Re-inject `plugin_excludes` into `global_excludes` after a replace merge.
    pub(crate) fn reapply_plugin_excludes(&mut self) {
        for ex in self.plugin_excludes.clone() {
            push_unique(&mut self.global_excludes, &ex);
        }
    }
}

fn push_unique(list: &mut Vec<String>, item: &str) {
    if !list.iter().any(|e| e == item) {
        list.push(item.to_string());
    }
}
