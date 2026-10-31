pub mod bundler;
pub mod factory_bot;
pub mod gemspec;
pub mod graphql;
pub mod layout;
pub mod lint;
pub mod metrics;
pub mod naming;
pub mod performance;
mod ported;
pub mod rails;
pub mod rake;
pub mod registry;
pub mod rspec;
pub mod rspec_rails;
pub mod security;
pub mod shared;
pub mod style;
pub mod walker;

use std::collections::HashMap;

use serde::Serialize;

use crate::diagnostic::{Diagnostic, Location, Severity};
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub use registry::CopRegistry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub enum EnabledState {
    True,
    False,
    Pending,
    #[default]
    Unset,
}

#[derive(Debug, Clone, Serialize)]
pub struct CopConfig {
    pub enabled: EnabledState,
    pub severity: Option<Severity>,
    pub exclude: Vec<String>,
    pub include: Vec<String>,
    pub options: HashMap<String, serde_yml::Value>,
}

impl Default for CopConfig {
    fn default() -> Self {
        Self {
            enabled: EnabledState::Unset,
            severity: None,
            exclude: Vec::new(),
            include: Vec::new(),
            options: HashMap::new(),
        }
    }
}

impl CopConfig {
    pub fn autocorrect_disabled(&self) -> bool {
        self.options
            .get("AutoCorrect")
            .is_some_and(|v| v.as_bool() == Some(false) || v.as_str() == Some("disabled"))
    }

    pub fn get_str<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.options
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or(default)
    }

    pub fn get_bool(&self, key: &str, default: bool) -> bool {
        self.options
            .get(key)
            .and_then(|v| v.as_bool())
            .unwrap_or(default)
    }

    pub fn get_usize(&self, key: &str, default: usize) -> usize {
        self.options
            .get(key)
            .and_then(|v| {
                v.as_u64()
                    .map(|u| u as usize)
                    .or_else(|| v.as_i64().map(|i| i as usize))
                    .or_else(|| v.as_f64().map(|f| f as usize))
            })
            .unwrap_or(default)
    }

    pub fn get_f64(&self, key: &str, default: f64) -> f64 {
        self.options
            .get(key)
            .and_then(|v| v.as_f64().or_else(|| v.as_u64().map(|u| u as f64)))
            .unwrap_or(default)
    }
}

pub trait Cop: Send + Sync {
    fn name(&self) -> &'static str;

    fn default_severity(&self) -> Severity {
        Severity::Convention
    }

    fn default_include(&self) -> &'static [&'static str] {
        &[]
    }

    fn default_exclude(&self) -> &'static [&'static str] {
        &[]
    }

    fn default_enabled(&self) -> bool {
        true
    }

    fn diagnostic(
        &self,
        source: &SourceFile,
        line: usize,
        column: usize,
        message: String,
    ) -> Diagnostic {
        Diagnostic {
            path: source.path_str().to_string(),
            location: Location { line, column },
            severity: self.default_severity(),
            cop_name: self.name().to_string(),
            message,
            corrected: false,
            correctable: self.supports_autocorrect(),
            source_line: String::new(),
            highlight_length: 1,
        }
    }

    fn supports_autocorrect(&self) -> bool {
        false
    }

    fn safe_autocorrect(&self) -> bool {
        true
    }

    /// Opt into the shared line phase (engine skips no-op line cops).
    fn uses_line_phase(&self) -> bool {
        false
    }

    #[allow(unused_variables)]
    fn check_lines(
        &self,
        source: &SourceFile,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
    }

    /// Opt into the shared source phase (engine skips no-op source cops).
    fn uses_source_phase(&self) -> bool {
        false
    }

    /// When true, engine builds one shared [`crate::model::FileModel`] and
    /// calls [`Self::check_file_model`] instead of [`Self::check_source`].
    fn needs_file_model(&self) -> bool {
        false
    }

    #[allow(unused_variables)]
    fn check_source(
        &self,
        source: &SourceFile,
        tree: &tree_sitter::Tree,
        code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
    }

    /// Shared FileModel pass — used by variable/metrics cops that would
    /// otherwise each rebuild the model. Tree is available as `file_model.tree`.
    #[allow(unused_variables)]
    fn check_file_model(
        &self,
        source: &SourceFile,
        file_model: &crate::model::FileModel<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
    }

    /// tree-sitter node kinds this cop cares about.
    /// Empty means the cop is not registered in the AST walker (line/source only).
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &[]
    }

    /// Method names that gate `call` / `command` / `command_call` dispatch.
    /// Empty (default) = all such nodes. Non-empty = only matching method names.
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[]
    }

    #[allow(unused_variables)]
    fn check_node(
        &self,
        source: &SourceFile,
        node: tree_sitter::Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
    }

    /// Post-pass hook after all cops ran (before disable filtering).
    #[allow(unused_variables)]
    fn audit_after_cops(
        &self,
        source: &SourceFile,
        offenses: &[Diagnostic],
        active: &[(&dyn Cop, &CopConfig)],
        diagnostics: &mut Vec<Diagnostic>,
        corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
    }

    /// When false, `Lint/RedundantCopDisableDirective` skips this cop (parity gaps).
    fn redundant_disable_audit(&self) -> bool {
        true
    }
}

/// True when the lint engine can invoke this cop (line, source, model, or AST phases).
pub fn cop_ran_in_lint(c: &dyn Cop) -> bool {
    c.uses_line_phase()
        || c.uses_source_phase()
        || c.needs_file_model()
        || !c.interested_node_kinds().is_empty()
}
