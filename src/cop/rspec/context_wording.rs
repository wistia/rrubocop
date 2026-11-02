use crate::cop::rspec::helpers::{RSPEC_INCLUDE, bare_rspec_call, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::node_text;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct ContextWording;
impl Cop for ContextWording {
    fn name(&self) -> &'static str {
        "RSpec/ContextWording"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "identifier"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<Correction>>,
    ) {
        if safe_send(source, node) {
            return;
        }
        if !matches!(
            bare_rspec_call(source, node),
            Some(b"context" | b"shared_context")
        ) || call_block(node).is_none()
        {
            return;
        }
        let Some(argument) = call_args(node).first().copied() else {
            return;
        };
        if !matches!(argument.kind(), "string" | "chained_string" | "subshell") {
            return;
        }
        let raw = string_options(config, "AllowedPatterns");
        let prefixes = string_options(config, "Prefixes");
        if raw.is_empty() && prefixes.is_empty() {
            return;
        }
        let text = crate::cop::ported::static_literal(source, argument).unwrap_or_else(|| {
            code_children(argument)
                .iter()
                .map(|child| node_text(source, *child))
                .collect()
        });
        let patterns: Vec<_> = raw
            .iter()
            .cloned()
            .chain(
                prefixes
                    .iter()
                    .map(|prefix| format!(r"^{}\b", regex::escape(prefix))),
            )
            .collect();
        if patterns
            .iter()
            .any(|pattern| regex::Regex::new(pattern).is_ok_and(|pattern| pattern.is_match(&text)))
        {
            return;
        }
        let mut descriptions: Vec<_> = raw
            .iter()
            .map(|pattern| {
                format!(
                    "/{}/",
                    &ruby_string(pattern)[1..ruby_string(pattern).len() - 1]
                )
            })
            .chain(
                prefixes
                    .iter()
                    .map(|prefix| format!(r"/^{}\b/", ruby_regexp_escape(prefix))),
            )
            .collect();
        if descriptions.len() > 1 {
            let last = descriptions.pop().unwrap();
            descriptions.push(format!("or {last}"));
        }
        report(
            self,
            source,
            argument.start_byte(),
            argument.end_byte(),
            format!(
                "Context description should match {}.",
                descriptions.join(", ")
            ),
            diagnostics,
        );
    }
}
