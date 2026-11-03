use crate::cop::rspec::helpers::{RSPEC_INCLUDE, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::rspec::separation::numbered_block;
use crate::cop::shared::call_receiver;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct IsExpectedSpecify;
impl Cop for IsExpectedSpecify {
    fn name(&self) -> &'static str {
        "RSpec/IsExpectedSpecify"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if safe_send(source, node) {
            return;
        }
        if name(source, node) != Some(b"specify")
            || !call_args(node).is_empty()
            || numbered_block(source, node)
            || node.start_position().row != node.end_position().row
        {
            return;
        }
        let Some(body) = call_block(node).and_then(|block| block.child_by_field_name("body"))
        else {
            return;
        };
        let statements = code_children(body);
        if statements.len() != 1 {
            return;
        }
        let expectation = statements[0];
        if safe_send(source, expectation) {
            return;
        }
        let Some(receiver) = call_receiver(expectation) else {
            return;
        };
        if safe_send(source, receiver)
            || !matches!(
                name(source, receiver),
                Some(b"is_expected" | b"are_expected")
            )
        {
            return;
        }
        let target = selector(node);
        let index = report(
            self,
            source,
            target.start_byte(),
            target.end_byte(),
            "Use `it` instead of `specify`.".into(),
            diagnostics,
        );
        if preserving_replace(
            source,
            node,
            &mut corrections,
            target.start_byte(),
            target.end_byte(),
            "it",
            self.name(),
        ) {
            diagnostics[index].corrected = true;
        }
    }
}
