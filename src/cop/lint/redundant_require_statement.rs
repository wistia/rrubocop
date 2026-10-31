//! Lint/RedundantRequireStatement — `require` of always-loaded stdlib features.

use tree_sitter::Node;

use crate::cop::shared::{argument_nodes, call_method_name, call_receiver, node_text};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::{Diagnostic, Severity};
use crate::parse::source::SourceFile;

pub struct RedundantRequireStatement;

fn string_feature(source: &SourceFile, node: Node<'_>) -> Option<String> {
    if node.kind() != "string" {
        return None;
    }
    // RuboCop matches `str` only (plain quotes), not interpolation / percent literals.
    let mut cur = node.walk();
    let kids: Vec<_> = node.named_children(&mut cur).collect();
    if kids.len() != 1 || kids[0].kind() != "string_content" {
        return None;
    }
    Some(node_text(source, kids[0]))
}

fn redundant_feature(name: &str, ruby: f64) -> bool {
    match name {
        "enumerator" => true,
        "thread" => ruby >= 2.1,
        "rational" | "complex" => ruby >= 2.2,
        "ruby2_keywords" => ruby >= 2.7,
        "fiber" => ruby >= 3.1,
        "set" => ruby >= 3.2,
        "pathname" => ruby >= 4.0,
        _ => false,
    }
}

fn redundant_require(source: &SourceFile, node: Node<'_>, ruby: f64) -> bool {
    if call_method_name(source, node) != Some(b"require") || call_receiver(node).is_some() {
        return false;
    }
    argument_nodes(node)
        .into_iter()
        .next()
        .and_then(|arg| string_feature(source, arg))
        .is_some_and(|name| redundant_feature(&name, ruby))
}

fn line_span(source: &SourceFile, node: Node<'_>) -> (usize, usize) {
    let mut start = node.start_byte();
    let mut end = node.end_byte();
    let bytes = source.as_bytes();
    if end < bytes.len() && bytes[end] == b'\n' {
        end += 1;
    } else if start > 0 && bytes[start - 1] == b'\n' {
        start -= 1;
    }
    (start, end)
}

impl Cop for RedundantRequireStatement {
    fn name(&self) -> &'static str {
        "Lint/RedundantRequireStatement"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }

    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"require"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if !redundant_require(source, node, config.get_f64("TargetRubyVersion", 2.7)) {
            return;
        }
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut diag = self.diagnostic(
            source,
            line,
            col,
            "Remove unnecessary `require` statement.".to_string(),
        );
        if let Some(corr) = corrections.as_mut() {
            let (start, end) = line_span(source, node);
            corr.push(Correction {
                start,
                end,
                replacement: String::new(),
                cop_name: self.name(),
                cop_index: 0,
                owner: None,
            });
            diag.corrected = true;
        }
        diagnostics.push(diag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::run_cop_full_with_config;
    use std::collections::HashMap;

    crate::cop_fixture_tests!(
        RedundantRequireStatement,
        "cops/lint/redundant_require_statement"
    );

    fn ruby_config(ver: f64) -> CopConfig {
        CopConfig {
            options: HashMap::from([(
                "TargetRubyVersion".into(),
                serde_yml::Value::Number(serde_yml::Number::from(ver)),
            )]),
            ..CopConfig::default()
        }
    }

    #[test]
    fn set_require_offense_on_ruby_32() {
        let diags = run_cop_full_with_config(
            &RedundantRequireStatement,
            b"require 'set'\n",
            ruby_config(3.2),
        );
        assert_eq!(diags.len(), 1, "{diags:?}");
    }

    #[test]
    fn set_require_ok_before_ruby_32() {
        let diags = run_cop_full_with_config(
            &RedundantRequireStatement,
            b"require 'set'\n",
            ruby_config(3.1),
        );
        assert!(diags.is_empty(), "{diags:?}");
    }
}
