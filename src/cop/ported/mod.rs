//! Additional pinned Ruby policies, including opt-in Wistia extension rules.
mod context;
mod lint_calls;
mod lint_duplicates;
mod lint_flow;
mod lint_literals;
mod retained;
mod style_blocks;
mod style_calls;
mod style_conditions;
mod style_regexp;
mod wistia_calls;
mod wistia_runtime;
mod wistia_scopes;

use crate::cop::{Cop, CopConfig, CopRegistry};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use context::{Context, Finding};
use tree_sitter::Node;

pub(super) struct Rule {
    name: &'static str,
    kinds: &'static [&'static str],
    check: fn(&Context<'_>, Node<'_>) -> Vec<Finding>,
    correctable: bool,
    safe: bool,
    extension: bool,
}

impl Cop for Rule {
    fn name(&self) -> &'static str {
        self.name
    }
    fn default_enabled(&self) -> bool {
        !self.extension
    }
    fn default_include(&self) -> &'static [&'static str] {
        if self.name.starts_with("AnyCable/") {
            &["**/channels/**/*.rb"]
        } else {
            &[]
        }
    }
    fn default_severity(&self) -> crate::diagnostic::Severity {
        if self.name.starts_with("Lint/") || self.name.starts_with("Security/") {
            crate::diagnostic::Severity::Warning
        } else {
            crate::diagnostic::Severity::Convention
        }
    }
    fn supports_autocorrect(&self) -> bool {
        self.correctable
    }
    fn safe_autocorrect(&self) -> bool {
        self.safe
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        if self.name == "Style/TopLevelMethodDefinition" {
            &["call", "identifier", "method", "singleton_method"]
        } else if bare_name(self.name).is_some() {
            &["call", "identifier"]
        } else {
            self.kinds
        }
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let context = Context { source, config };
        if node.kind() == "identifier"
            && bare_name(self.name).is_some()
            && !bare_call(self.name, &context, node)
        {
            return;
        }
        if context::safe(&context, node) && !accepts_safe(self.name) {
            return;
        }
        for mut finding in (self.check)(&context, node) {
            if config.autocorrect_disabled() {
                finding.edit = None;
                finding.additional.clear();
            }
            emit(self, source, finding, diagnostics, &mut corrections);
        }
    }
}

fn bare_name(name: &str) -> Option<&str> {
    match name {
        "Cops/NoSleep" => Some("sleep"),
        "Cops/NoForeignKeys" => Some("add_foreign_key"),
        "AnyCable/PeriodicalTimers" => Some("periodically"),
        "Style/TopLevelMethodDefinition" => Some("define_method"),
        _ => None,
    }
}
fn bare_call(name: &str, c: &Context<'_>, n: Node<'_>) -> bool {
    bare_name(name) == Some(c.text(n))
        && !identifier_slot(n)
        && !in_parameters(n)
        && !context::local_identifier(c, n)
}
fn identifier_slot(n: Node<'_>) -> bool {
    n.parent().is_some_and(|p| {
        context::field(p, "method") == Some(n)
            || context::field(p, "name") == Some(n)
            || context::field(p, "left") == Some(n)
    })
}
fn in_parameters(n: Node<'_>) -> bool {
    context::ancestors(n)
        .iter()
        .take_while(|p| !matches!(p.kind(), "method" | "block" | "do_block"))
        .any(|p| p.kind().contains("parameter"))
}

fn accepts_safe(name: &str) -> bool {
    matches!(
        name,
        "Cops/NoCallbackSkippingProtectedWrites"
            | "Lint/SelfAssignment"
            | "Lint/SafeNavigationConsistency"
            | "Lint/RedundantWithIndex"
            | "Lint/RedundantWithObject"
            | "Style/StringChars"
            | "Style/UnpackFirst"
            | "Style/RedundantSort"
            | "Style/HashExcept"
            | "Style/MapCompactWithConditionalBlock"
    )
}

fn emit(
    rule: &Rule,
    source: &SourceFile,
    finding: Finding,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let mut diagnostic = finding_diagnostic(rule, source, &finding);
    let before = corrections.as_ref().map_or(0, |edits| edits.len());
    if let Some((start, end, replacement)) = finding.edit {
        diagnostic.corrected =
            crate::cop::shared::push_replace(corrections, start, end, replacement, rule.name);
    }
    for (start, end, replacement) in finding.additional {
        diagnostic.corrected |=
            crate::cop::shared::push_replace(corrections, start, end, replacement, rule.name);
    }
    own_edits(corrections, before, finding.start);
    diagnostics.push(diagnostic);
}

fn finding_diagnostic(rule: &Rule, source: &SourceFile, finding: &Finding) -> Diagnostic {
    let (line, col) = source.offset_to_line_col(finding.start);
    let mut diagnostic = rule.diagnostic(source, line, col, finding.message.clone());
    diagnostic.highlight_length = finding.end.saturating_sub(finding.start).max(1);
    diagnostic.correctable = finding.edit.is_some();
    diagnostic
}

fn own_edits(corrections: &mut Option<&mut Vec<Correction>>, before: usize, owner: usize) {
    if let Some(edits) = corrections.as_deref_mut() {
        for edit in &mut edits[before..] {
            edit.owner = Some(owner);
        }
    }
}

pub fn register_all(registry: &mut CopRegistry) {
    retained::register(registry);
    lint_calls::register(registry);
    lint_duplicates::register(registry);
    lint_literals::register(registry);
    lint_flow::register(registry);
    style_calls::register(registry);
    style_blocks::register(registry);
    style_conditions::register(registry);
    style_regexp::register(registry);
    wistia_calls::register(registry);
    wistia_scopes::register(registry);
    wistia_runtime::register(registry);
}

macro_rules! rules {
    ($registry:expr; $(($name:literal, $kinds:expr, $check:path, $correctable:expr, $safe:expr, $extension:expr)),+ $(,)?) => {
        $( $registry.register(Box::new(super::Rule { name: $name, kinds: $kinds, check: $check, correctable: $correctable, safe: $safe, extension: $extension })); )+
    };
}
pub(super) use rules;

#[cfg(test)]
mod tests;
