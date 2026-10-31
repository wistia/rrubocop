//! Resolved RuboCop configuration struct and simple accessors.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::cop::{CopConfig, CopRegistry, EnabledState};

use super::standard::is_plugin_department;
use super::types::{ConfigLayer, DepartmentConfig, NewCopsPolicy};

#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    /// Per-cop configs keyed by cop name (e.g. "Style/FrozenStringLiteralComment")
    pub(crate) cop_configs: HashMap<String, CopConfig>,
    /// Department-level configs keyed by department name (e.g. "RSpec", "Rails")
    pub(crate) department_configs: HashMap<String, DepartmentConfig>,
    pub(crate) global_excludes: Vec<String>,
    /// Directory containing the resolved config file (for relative path resolution).
    pub(crate) config_dir: Option<PathBuf>,
    /// Directory being linted; nested `.rubocop.yml` discovery is scoped here,
    /// not under `config_dir` when config was found via walk-up (e.g. `~/.rubocop.yml`).
    pub(crate) scan_root: Option<PathBuf>,
    /// How to handle `Enabled: pending` cops.
    pub(crate) new_cops: NewCopsPolicy,
    /// When true, cops without explicit `Enabled: true` are disabled.
    pub(crate) disabled_by_default: bool,
    /// All cop names mentioned in `require:` gem default configs.
    /// Cops from plugin departments not in this set are treated as non-existent
    /// (the installed gem version doesn't include them).
    pub(crate) require_known_cops: HashSet<String>,
    /// Department names that had gems loaded via `require:`.
    pub(crate) require_departments: HashSet<String>,
    /// Target Ruby version from AllCops.TargetRubyVersion (e.g. 3.1, 3.2).
    /// None means not specified (cops should default to 2.7 per RuboCop convention).
    pub target_ruby_version: Option<f64>,
    pub(crate) rubocop_version: Option<String>,
    pub(crate) rubocop_rails_version: Option<String>,
    /// Target Rails version from AllCops.TargetRailsVersion (e.g. 7.1, 8.0).
    /// None means not specified (cops should default to 5.0 per RuboCop convention).
    pub(crate) target_rails_version: Option<f64>,
    /// Whether ActiveSupport extensions are enabled (AllCops.ActiveSupportExtensionsEnabled).
    /// Set to true by rubocop-rails. Affects cops like Style/CollectionQuerying.
    pub(crate) active_support_extensions_enabled: bool,
    /// All cop names found in the installed rubocop gem's config/default.yml.
    /// When non-empty, core cops (Layout, Lint, Style, etc.) not in this set
    /// are treated as non-existent in the project's rubocop version.
    pub(crate) rubocop_known_cops: HashSet<String>,
    /// Cops mentioned in the project config layer (inherit_from, inherit_gem,
    /// local config — but NOT from require: gem defaults).
    /// Used by department-level Enabled:false to distinguish user-explicit cops
    /// from rubocop default cops.
    pub(crate) project_mentioned_cops: HashSet<String>,
    /// Departments mentioned in the project config layer (inherit_from,
    /// inherit_gem, local config — but NOT from require: gem defaults).
    pub(crate) project_mentioned_depts: HashSet<String>,
    /// Departments that have `Enabled: true` explicitly in the project config.
    /// Distinguished from departments merely mentioned with other keys (e.g., Exclude).
    /// Used for DisabledByDefault: cops in these departments get their default
    /// enabled state restored (matching RuboCop's handle_disabled_by_default).
    pub(crate) project_enabled_depts: HashSet<String>,
    /// Per-directory config layers from nested `.rubocop.yml` files.
    /// Keyed by directory path (sorted deepest-first for lookup).
    pub(crate) dir_overrides: Vec<(PathBuf, ConfigLayer)>,
    /// Whether the `railties` gem was found in the project's Gemfile.lock.
    /// RuboCop 1.84+ uses `requires_gem 'railties'` to gate Rails cops — if
    /// `railties` is not in the lockfile, cops with `minimum_target_rails_version`
    /// are silently disabled regardless of `TargetRailsVersion` in config.
    pub(crate) railties_in_lockfile: bool,
    /// The actual `railties` gem version from Gemfile.lock (e.g. 4.2 for "4.2.11.3").
    /// When corpus runs force a higher `TargetRailsVersion`, some cops still need
    /// the real lockfile version to mirror RuboCop's `requires_gem 'railties', '>= x.y'`.
    pub(crate) railties_version: Option<f64>,
    /// The `rack` gem version from Gemfile.lock (e.g. 3.1 for "3.1.8").
    /// Used by `Rails/HttpStatusNameConsistency` and `RSpecRails/HttpStatusNameConsistency`
    /// which require `rack >= 3.1.0` (via RuboCop's `requires_gem 'rack', '>= 3.1.0'`).
    pub(crate) rack_version: Option<f64>,
    /// Base directory for resolving Include/Exclude path patterns.
    /// RuboCop's `base_dir_for_path_parameters`: if the config filename starts
    /// with `.rubocop`, this is the config file's parent (canonical). Otherwise
    /// (e.g., `baseline_rubocop.yml`), this is the current working directory.
    /// This distinction matters because non-dotfile configs use cwd-relative patterns.
    pub(crate) base_dir: Option<PathBuf>,
    /// AllCops.MigratedSchemaVersion from rubocop-rails.
    /// When set, files whose basename contains a 14-digit "timestamp" <= this value
    /// have ALL offenses suppressed (rubocop-rails' MigrationFileSkippable).
    /// Default sentinel from rubocop-rails: `'19700101000000'`.
    pub(crate) migrated_schema_version: Option<String>,
    /// AllCops.DisplayCopNames (default true).
    pub display_cop_names: bool,
    /// AllCops.DisplayStyleGuide (default false).
    pub display_style_guide: bool,
    /// AllCops.ExtraDetails (default false).
    pub extra_details: bool,
    /// AllCops.StyleGuideBaseURL.
    pub style_guide_base_url: Option<String>,
}

impl ResolvedConfig {
    pub fn empty() -> Self {
        Self {
            cop_configs: HashMap::new(),
            department_configs: HashMap::new(),
            global_excludes: Vec::new(),
            config_dir: None,
            scan_root: None,
            new_cops: NewCopsPolicy::Disable,
            disabled_by_default: false,
            require_known_cops: HashSet::new(),
            require_departments: HashSet::new(),
            target_ruby_version: None,
            rubocop_version: None,
            rubocop_rails_version: None,
            target_rails_version: None,
            active_support_extensions_enabled: false,
            rubocop_known_cops: HashSet::new(),
            project_mentioned_cops: HashSet::new(),
            project_mentioned_depts: HashSet::new(),
            project_enabled_depts: HashSet::new(),
            dir_overrides: Vec::new(),
            railties_in_lockfile: false,
            railties_version: None,
            rack_version: None,
            base_dir: None,
            migrated_schema_version: None,
            display_cop_names: true,
            display_style_guide: false,
            extra_details: false,
            style_guide_base_url: None,
        }
    }

    /// Apply CLI `-D` / `--no-display-cop-names` overrides.
    pub fn apply_display_cli(&mut self, display: Option<bool>) {
        if let Some(v) = display {
            self.display_cop_names = v;
        }
    }

    /// Register plugin departments so their cops are enabled during
    /// `build_cop_filters`. Used with `--force-default-config --only` to
    /// ensure plugin cops (RSpec, Rails, etc.) run in isolation.
    pub fn register_departments_from_only(&mut self, only: &[String]) {
        for cop_name in only {
            if let Some(dept) = cop_name.split('/').next() {
                if is_plugin_department(dept) {
                    self.require_departments.insert(dept.to_string());
                }
            }
        }
    }

    /// Whether this config has any directory-specific overrides (nested .rubocop.yml files).
    pub fn has_dir_overrides(&self) -> bool {
        !self.dir_overrides.is_empty()
    }

    /// Pre-compute base CopConfig for each cop in the registry (indexed by cop index).
    pub fn precompute_cop_configs(&self, registry: &CopRegistry) -> Vec<CopConfig> {
        registry
            .cops()
            .iter()
            .map(|cop| self.cop_config(cop.name()))
            .collect()
    }

    /// Global exclude patterns from AllCops.Exclude.
    pub fn global_excludes(&self) -> &[String] {
        &self.global_excludes
    }

    /// Directory containing the resolved config file.
    pub fn config_dir(&self) -> Option<&Path> {
        self.config_dir.as_deref()
    }

    /// Base directory for resolving Include/Exclude path patterns.
    /// Falls back to `config_dir` if not set.
    pub fn base_dir(&self) -> Option<&Path> {
        self.base_dir.as_deref().or(self.config_dir.as_deref())
    }

    pub(crate) fn nested_search_root(&self) -> Option<PathBuf> {
        self.scan_root.clone().or_else(|| self.config_dir.clone())
    }

    /// Return all cop names from the config that would be enabled given
    /// the current NewCops/DisabledByDefault settings.
    pub fn enabled_cop_names(&self) -> Vec<String> {
        self.cop_configs
            .iter()
            .filter(|(_name, config)| match config.enabled {
                EnabledState::True => true,
                EnabledState::Unset => !self.disabled_by_default,
                EnabledState::Pending => {
                    self.new_cops == NewCopsPolicy::Enable && !self.disabled_by_default
                }
                EnabledState::False => false,
            })
            .map(|(name, _)| name.clone())
            .collect()
    }
}
