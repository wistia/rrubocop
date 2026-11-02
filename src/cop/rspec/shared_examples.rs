use crate::cop::rspec::helpers::{RSPEC_INCLUDE, bare_rspec_call};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::call_receiver;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct SharedExamples;
impl Cop for SharedExamples {
    fn name(&self) -> &'static str {
        "RSpec/SharedExamples"
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
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if safe_send(source, node) {
            return;
        }
        let Some(method) = name(source, node) else {
            return;
        };
        let shared = [
            b"shared_context".as_slice(),
            b"shared_examples",
            b"shared_examples_for",
        ]
        .contains(&method)
            && bare_rspec_call(source, node).is_some();
        let included = [
            b"it_behaves_like".as_slice(),
            b"it_should_behave_like",
            b"include_examples",
            b"include_context",
        ]
        .contains(&method)
            && call_receiver(node).is_none();
        if !shared && !included {
            return;
        }
        let Some(argument) = call_args(node).first().copied() else {
            return;
        };
        let style = config.get_str("EnforcedStyle", "string");
        if !(style == "symbol" && argument.kind() == "string"
            || style == "string" && symbol_node(argument))
        {
            return;
        }
        let Some(value) = crate::cop::ported::static_literal(source, argument) else {
            return;
        };
        let (replacement, current, action) = if style == "symbol" {
            (
                format!(":{}", value.to_lowercase().replace(' ', "_")),
                ruby_string(&value),
                "symbolize",
            )
        } else {
            (
                format!("'{}'", value.replace('_', " ")),
                ruby_symbol(&value),
                "titleize",
            )
        };
        let index = report(
            self,
            source,
            argument.start_byte(),
            argument.end_byte(),
            format!("Prefer {replacement} over `{current}` to {action} shared examples."),
            diagnostics,
        );
        let expected_value = if style == "symbol" {
            value.to_lowercase().replace(' ', "_")
        } else {
            value.replace('_', " ")
        };
        if !literal_replacement(&replacement, &expected_value, style == "symbol") {
            return;
        }
        if preserving_replace(
            source,
            node,
            &mut corrections,
            argument.start_byte(),
            argument.end_byte(),
            replacement,
            self.name(),
        ) {
            diagnostics[index].corrected = true;
        }
    }
}
