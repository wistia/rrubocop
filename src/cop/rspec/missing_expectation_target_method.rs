use crate::cop::rspec::helpers::RSPEC_INCLUDE;
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::call_receiver;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct MissingExpectationTargetMethod;
impl Cop for MissingExpectationTargetMethod {
    fn name(&self) -> &'static str {
        "RSpec/MissingExpectationTargetMethod"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "identifier", "binary", "unary", "element_reference"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<Correction>>,
    ) {
        if runner(source, node) {
            return;
        }
        let Some(receiver) = call_receiver(node)
            .or_else(|| node.child_by_field_name("left"))
            .or_else(|| node.child_by_field_name("operand"))
            .or_else(|| node.child_by_field_name("object"))
        else {
            return;
        };
        if !bare(source, receiver, b"expect")
            && !(bare(source, receiver, b"is_expected") && call_args(receiver).is_empty())
        {
            return;
        }
        let target = if matches!(node.kind(), "binary" | "unary") {
            node.child_by_field_name("operator").unwrap_or(node)
        } else {
            selector(node)
        };
        report(
            self,
            source,
            target.start_byte(),
            target.end_byte(),
            "Use `.to`, `.not_to` or `.to_not` to set an expectation.".into(),
            diagnostics,
        );
    }
}
