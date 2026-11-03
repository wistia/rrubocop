//! Naming/VariableName — locals/ivars snake_case (EnforcedStyle).
//!
//! RuboCop aliases `on_lvar` → `on_lvasgn`, so reads of pattern-bound locals
//! (`applyTime` after `=> { applyTime: }`) are flagged; intros via `match_var`
//! are not.

use std::sync::{Arc, Mutex};

use std::sync::LazyLock;

use regex::Regex;
use tree_sitter::Node;

use crate::cop::shared::node_bytes;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::model::IntroKind;
use crate::parse::source::SourceFile;

pub struct VariableName;

// Locals, `@ivar`, `@@cvar` — RuboCop strips sigils when matching style.
static SNAKE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\A(?:@@|@)?[\p{Ll}\d_]+[!?=]?\z").unwrap());

fn bare_name(raw: &str) -> &str {
    raw.strip_prefix("@@")
        .or_else(|| raw.strip_prefix('@'))
        .unwrap_or(raw)
}

fn style_ok(name: &str, style: &str) -> bool {
    if name == "_" {
        return true;
    }
    match style {
        "camelCase" => is_lower_camel(name),
        _ => SNAKE.is_match(name),
    }
}

fn is_lower_camel(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return true;
    };
    let start = if first == '_' {
        match chars.next() {
            None => return true,
            Some(c) => c,
        }
    } else {
        first
    };
    if !start.is_lowercase() {
        return false;
    }
    chars.all(|c| c.is_lowercase() || c.is_uppercase() || c.is_ascii_digit())
}

fn string_list(config: &CopConfig, key: &str) -> Vec<String> {
    match config.options.get(key) {
        Some(serde_yml::Value::Sequence(items)) => items
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        Some(serde_yml::Value::String(s)) => vec![s.clone()],
        _ => Vec::new(),
    }
}

const OPTION_KEYS: [&str; 4] = [
    "AllowedIdentifiers",
    "AllowedPatterns",
    "ForbiddenIdentifiers",
    "ForbiddenPatterns",
];
struct NameOptions {
    values: [Option<serde_yml::Value>; 4],
    allowed_identifiers: Vec<String>,
    allowed_patterns: Vec<Regex>,
    forbidden_identifiers: Vec<String>,
    forbidden_patterns: Vec<Regex>,
}
fn name_options(source: &SourceFile, config: &CopConfig) -> Arc<NameOptions> {
    struct Settings(Mutex<Vec<Arc<NameOptions>>>);
    let settings = source.cached(0, || Settings(Default::default()));
    let mut options = settings.0.lock().unwrap();
    if let Some(existing) = options.iter().find(|entry| {
        OPTION_KEYS
            .iter()
            .enumerate()
            .all(|(index, key)| entry.values[index].as_ref() == config.options.get(*key))
    }) {
        return existing.clone();
    }
    let compile = |key| {
        string_list(config, key)
            .into_iter()
            .filter_map(|pattern| Regex::new(&pattern).ok())
            .collect()
    };
    let result = Arc::new(NameOptions {
        values: OPTION_KEYS.map(|key| config.options.get(key).cloned()),
        allowed_identifiers: string_list(config, "AllowedIdentifiers"),
        allowed_patterns: compile("AllowedPatterns"),
        forbidden_identifiers: string_list(config, "ForbiddenIdentifiers"),
        forbidden_patterns: compile("ForbiddenPatterns"),
    });
    options.push(result.clone());
    result
}
fn allowed(options: &NameOptions, name: &str) -> bool {
    options.allowed_identifiers.iter().any(|a| a == name)
        || options
            .allowed_patterns
            .iter()
            .any(|pattern| pattern.is_match(name))
}

fn forbidden_msg(name: &str) -> String {
    format!("`{name}` is forbidden, use another variable name instead.")
}

fn check_forbidden(
    cop: &VariableName,
    source: &SourceFile,
    name: &str,
    line: usize,
    column: usize,
    options: &NameOptions,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if options.forbidden_identifiers.iter().any(|f| f == name) {
        diagnostics.push(cop.diagnostic(source, line, column, forbidden_msg(name)));
    }
    for pattern in &options.forbidden_patterns {
        if pattern.is_match(name) {
            diagnostics.push(cop.diagnostic(source, line, column, forbidden_msg(name)));
        }
    }
}

fn check_name_str(
    cop: &VariableName,
    source: &SourceFile,
    name: &str,
    line: usize,
    column: usize,
    config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let name = bare_name(name);
    let configured_names = OPTION_KEYS
        .iter()
        .any(|key| match config.options.get(*key) {
            Some(serde_yml::Value::String(_)) => true,
            Some(serde_yml::Value::Sequence(values)) => {
                values.iter().any(|value| value.as_str().is_some())
            }
            _ => false,
        });
    if configured_names {
        let options = name_options(source, config);
        check_forbidden(cop, source, name, line, column, &options, diagnostics);
        if allowed(&options, name) {
            return;
        }
    }
    let style = config.get_str("EnforcedStyle", "snake_case");
    if style_ok(name, style) {
        return;
    }
    let style_msg = if style == "camelCase" {
        "camelCase"
    } else {
        "snake_case"
    };
    diagnostics.push(cop.diagnostic(
        source,
        line,
        column,
        format!("Use {style_msg} for variable names."),
    ));
}

fn check_name(
    cop: &VariableName,
    source: &SourceFile,
    name_node: Node<'_>,
    config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let raw = String::from_utf8_lossy(node_bytes(source, name_node));
    let (line, column) = source.offset_to_line_col(name_node.start_byte());
    check_name_str(cop, source, &raw, line, column, config, diagnostics);
}

fn check_entry(
    cop: &VariableName,
    source: &SourceFile,
    file_model: &crate::model::FileModel<'_>,
    name: &str,
    entry: &crate::model::Entry,
    config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for byte in entry_check_bytes(entry) {
        let (line, column) = file_model.line_col(byte);
        check_name_str(cop, source, name, line, column, config, diagnostics);
    }
}

fn entry_check_bytes(entry: &crate::model::Entry) -> Vec<usize> {
    let mut bytes: Vec<_> = entry.reads.iter().map(|r| r.byte).collect();
    if entry.writes.is_empty() {
        if entry.intro_kind != IntroKind::Pattern {
            bytes.push(entry.intro_byte);
        }
        return bytes;
    }
    for w in &entry.writes {
        if entry.intro_kind == IntroKind::Pattern && w.byte == entry.intro_byte {
            continue;
        }
        bytes.push(w.byte);
    }
    bytes
}

impl Cop for VariableName {
    fn name(&self) -> &'static str {
        "Naming/VariableName"
    }

    fn needs_file_model(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        // Locals via FileModel; ivars/cvars are not in the model.
        &["assignment", "operator_assignment"]
    }

    fn check_file_model(
        &self,
        source: &SourceFile,
        file_model: &crate::model::FileModel<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        for scope in file_model.scopes.iter() {
            for (name, entry) in &scope.entries {
                check_entry(self, source, file_model, name, entry, config, diagnostics);
            }
        }
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        match node.kind() {
            "assignment" | "operator_assignment" => {
                let Some(left) = node.child_by_field_name("left") else {
                    return;
                };
                if matches!(left.kind(), "instance_variable" | "class_variable") {
                    check_name(self, source, left, config, diagnostics);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(VariableName, "cops/naming/variable_name");
    #[test]
    fn option_cache_tracks_value_changes_and_keeps_duplicate_forbidden_matches() {
        let source = SourceFile::from_bytes("names.rb", vec![]);
        let mut config = CopConfig::default();
        config.options.insert(
            "AllowedPatterns".into(),
            serde_yml::from_str("['^Foo$', '[']").unwrap(),
        );
        let first = name_options(&source, &config);
        assert!(allowed(&first, "Foo"));
        assert!(Arc::ptr_eq(&first, &name_options(&source, &config)));
        config.options.insert(
            "AllowedPatterns".into(),
            serde_yml::from_str("['^Bar$']").unwrap(),
        );
        config.options.insert(
            "ForbiddenPatterns".into(),
            serde_yml::from_str("['^Foo$', '^F', '[']").unwrap(),
        );
        let next = name_options(&source, &config);
        assert!(!allowed(&next, "Foo"));
        assert!(allowed(&next, "Bar"));
        let mut diagnostics = Vec::new();
        check_forbidden(&VariableName, &source, "Foo", 1, 0, &next, &mut diagnostics);
        assert_eq!(diagnostics.len(), 2);
    }
}
