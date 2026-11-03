use crate::cop::rspec::helpers::{RSPEC_INCLUDE, bare_rspec_call, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::node_text;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

#[derive(Clone)]
struct WordingPatterns {
    patterns: std::sync::Arc<Vec<regex::Regex>>,
    description: String,
    empty: bool,
}
fn wording_patterns(source: &SourceFile, config: &CopConfig) -> WordingPatterns {
    super::cache::config_fact(source, config, &["AllowedPatterns", "Prefixes"], || {
        let raw = string_options(config, "AllowedPatterns");
        let prefixes = string_options(config, "Prefixes");
        let patterns = raw
            .iter()
            .cloned()
            .chain(
                prefixes
                    .iter()
                    .map(|prefix| format!(r"^{}\b", regex::escape(prefix))),
            )
            .filter_map(|pattern| regex::Regex::new(&pattern).ok())
            .collect();
        let mut descriptions: Vec<_> = raw
            .iter()
            .map(|pattern| {
                let quoted = ruby_string(pattern);
                format!("/{}/", &quoted[1..quoted.len() - 1])
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
        WordingPatterns {
            patterns: std::sync::Arc::new(patterns),
            description: descriptions.join(", "),
            empty: raw.is_empty() && prefixes.is_empty(),
        }
    })
}
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
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"context", b"shared_context"]
    }
    fn interested_identifier_names(&self) -> &'static [&'static [u8]] {
        self.interested_call_names()
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
        let prepared = wording_patterns(source, config);
        if prepared.empty {
            return;
        }
        let text = crate::cop::ported::static_literal(source, argument).unwrap_or_else(|| {
            code_children(argument)
                .iter()
                .map(|child| node_text(source, *child))
                .collect()
        });
        if prepared
            .patterns
            .iter()
            .any(|pattern| pattern.is_match(&text))
        {
            return;
        }
        report(
            self,
            source,
            argument.start_byte(),
            argument.end_byte(),
            format!("Context description should match {}.", prepared.description),
            diagnostics,
        );
    }
}
