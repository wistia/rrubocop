//! CopConfig resolution with AllCops / sibling option injection.

use crate::cop::CopConfig;

use super::ResolvedConfig;
use super::resolved_inject::{
    inject_active_support, inject_config_disabled_cops, inject_end_alignment,
    inject_first_argument, inject_first_hash_indent, inject_globals, inject_hash_alignment,
    inject_indentation_width, inject_line_length, inject_missing_else, inject_non_nil_check,
    inject_operator_spacing, inject_quoted_symbols, inject_rack_version,
    inject_redundant_line_break, inject_redundant_parentheses, inject_rescue_ensure_alignment,
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
        let nullable_safety: &[&str] = match name {
            "Style/TrailingCommaInBlockArgs" => &["SafeAutoCorrect"],
            "Lint/TrailingCommaInAttributeDeclaration" => &["Safe", "SafeAutoCorrect"],
            _ => &[],
        };
        for key in nullable_safety {
            if config.options.get(*key) == Some(&serde_yml::Value::Null) {
                config.options.insert((*key).into(), false.into());
            }
        }
        self.inject_indentation_width_fallback(name, config);
        inject_rack_version(self, name, config);
        inject_line_length(self, name, config);
        inject_redundant_line_break(self, name, config);
        inject_active_support(self, name, config);
        inject_hash_alignment(self, name, config);
        inject_operator_spacing(self, name, config);
        inject_redundant_parentheses(self, name, config);
        inject_non_nil_check(self, name, config);
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

    fn inject_indentation_width_fallback(&self, name: &str, config: &mut CopConfig) {
        if !matches!(
            name,
            "Style/TrailingBodyOnMethodDefinition"
                | "Style/ComparableClamp"
                | "Layout/ArrayAlignment"
                | "Layout/ParameterAlignment"
                | "Layout/ArgumentAlignment"
        ) {
            return;
        }
        if name != "Style/TrailingBodyOnMethodDefinition"
            && config
                .options
                .get("IndentationWidth")
                .is_some_and(|value| value.as_u64().is_some())
        {
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

#[cfg(test)]
mod alignment_width_tests {
    use super::*;
    #[test]
    fn cop_numeric_width_precedes_global_and_nil_falls_back() {
        for name in [
            "Layout/ArrayAlignment",
            "Layout/ParameterAlignment",
            "Layout/ArgumentAlignment",
        ] {
            for (own, global, expected) in [
                (None, Some(4), 4),
                (Some(serde_yml::Value::Null), Some(4), 4),
                (Some(0.into()), Some(4), 0),
                (Some(2.into()), Some(4), 2),
                (None, Some(0), 0),
                (None, None, 2),
            ] {
                let mut config = ResolvedConfig::empty();
                let mut specific = CopConfig::default();
                if let Some(own) = own {
                    specific.options.insert("IndentationWidth".into(), own);
                }
                config.cop_configs.insert(name.into(), specific);
                config.cop_configs.insert(
                    "Layout/IndentationWidth".into(),
                    CopConfig {
                        options: [(
                            "Width".into(),
                            global.map_or(serde_yml::Value::Null, serde_yml::Value::from),
                        )]
                        .into(),
                        ..CopConfig::default()
                    },
                );
                assert_eq!(
                    config.cop_config(name).get_usize("IndentationWidth", 2),
                    expected,
                    "{name}"
                );
            }
        }
    }
}

#[cfg(test)]
mod argument_hash_style_tests {
    use super::*;
    #[test]
    fn compatibility_options_preserve_style_arrays_and_disabled_state() {
        let mut config = ResolvedConfig::empty();
        let styles = serde_yml::Value::Sequence(vec!["key".into(), "separator".into()]);
        config.cop_configs.insert(
            "Layout/HashAlignment".into(),
            CopConfig {
                enabled: crate::cop::EnabledState::False,
                options: [
                    ("EnforcedColonStyle".into(), styles.clone()),
                    ("EnforcedHashRocketStyle".into(), "separator".into()),
                ]
                .into(),
                ..CopConfig::default()
            },
        );
        let resolved = config.cop_config("Layout/ArgumentAlignment");
        assert!(!resolved.get_bool("HashAlignmentEnabled", true));
        assert_eq!(resolved.options["HashAlignmentEnforcedColonStyle"], styles);
        assert_eq!(
            resolved.get_str("HashAlignmentEnforcedHashRocketStyle", "key"),
            "separator"
        );
    }
}
