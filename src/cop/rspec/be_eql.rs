use crate::cop::rspec::helpers::RSPEC_INCLUDE;
use crate::cop::rspec::parity_helpers::*;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct BeEql;
impl Cop for BeEql {
    fn name(&self) -> &'static str {
        "RSpec/BeEql"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "identifier"]
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn safe_autocorrect(&self) -> bool {
        false
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let _ = config;
        if safe_send(source, node) || name(source, node) != Some(b"to") {
            return;
        }
        let args = call_args(node);
        if args.len() != 1 {
            return;
        }
        let matcher = args[0];
        if safe_send(source, matcher) || !bare(source, matcher, b"eql") {
            return;
        }
        let values = call_args(matcher);
        if values.len() != 1 {
            return;
        }
        let value = values[0];
        let identity = matches!(value.kind(), "true" | "false" | "nil" | "integer" | "float")
            || symbol_node(value) && crate::cop::ported::static_literal(source, value).is_some()
            || value.kind() == "unary"
                && code_children(value)
                    .last()
                    .is_some_and(|value| matches!(value.kind(), "integer" | "float"));
        if !identity {
            return;
        }
        let message = "Prefer `be` over `eql`.".into();
        let replacement = "be";
        let target = selector(matcher);
        let index = report(
            self,
            source,
            target.start_byte(),
            target.end_byte(),
            message,
            diagnostics,
        );
        if preserving_replace(
            source,
            matcher,
            &mut corrections,
            target.start_byte(),
            target.end_byte(),
            replacement,
            self.name(),
        ) {
            diagnostics[index].corrected = true;
        }
    }
}
