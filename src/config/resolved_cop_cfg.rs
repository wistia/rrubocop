//! CopConfig resolution with AllCops / sibling option injection.

use crate::cop::CopConfig;

use super::ResolvedConfig;
use super::resolved_inject::{
    inject_active_support, inject_config_disabled_cops, inject_end_alignment,
    inject_first_argument, inject_first_hash_indent, inject_globals, inject_hash_alignment,
    inject_indentation_width, inject_line_length, inject_missing_else, inject_quoted_symbols,
    inject_rack_version, inject_redundant_line_break, inject_rescue_ensure_alignment,
    inject_space_after_comma,
};

impl ResolvedConfig {
    /// Get the resolved config for a specific cop.
    ///
    /// Injects global AllCops settings and sibling-cop options so individual
    /// cops can access them without special plumbing.
    pub fn cop_config(&self, name: &str) -> CopConfig {
        let mut config = self.cop_configs.get(name).cloned().unwrap_or_default();
        self.inject_cop_options(name, &mut config);
        config
    }

    fn inject_cop_options(&self, name: &str, config: &mut CopConfig) {
        inject_globals(self, config);
        if name == "Performance/Detect" {
            if let Some(methods) = self
                .cop_configs
                .get("Style/CollectionMethods")
                .and_then(|sibling| sibling.options.get("PreferredMethods"))
            {
                config.options.insert(
                    "PreferredDetectMethod".into(),
                    methods
                        .get("detect")
                        .cloned()
                        .unwrap_or(serde_yml::Value::Null),
                );
            }
        }
        self.inject_method_body_width(name, config);
        inject_rack_version(self, name, config);
        inject_line_length(self, name, config);
        inject_redundant_line_break(self, name, config);
        inject_active_support(self, name, config);
        inject_hash_alignment(self, name, config);
        inject_first_argument(self, name, config);
        inject_first_hash_indent(self, name, config);
        inject_end_alignment(self, name, config);
        inject_rescue_ensure_alignment(self, name, config);
        inject_indentation_width(self, name, config);
        inject_space_after_comma(self, name, config);
        inject_missing_else(self, name, config);
        inject_quoted_symbols(self, name, config);
        inject_config_disabled_cops(self, name, config);
    }

    fn inject_method_body_width(&self, name: &str, config: &mut CopConfig) {
        if name != "Style/TrailingBodyOnMethodDefinition" {
            return;
        }
        if let Some(width) = self
            .cop_configs
            .get("Layout/IndentationWidth")
            .and_then(|c| c.options.get("Width"))
        {
            config
                .options
                .insert("IndentationWidth".into(), width.clone());
        }
    }

    /// Get the resolved config for a cop, applying directory-specific overrides
    /// based on the file path.
    pub fn cop_config_for_file(&self, name: &str, file_path: &std::path::Path) -> CopConfig {
        self.effective_config_for_file(file_path)
            .map_or_else(|| self.cop_config(name), |config| config.cop_config(name))
    }
}
