use crate::cop::rspec::helpers::RSPEC_INCLUDE;
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::{call_receiver, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct IdenticalEqualityAssertion;
impl Cop for IdenticalEqualityAssertion {
    fn name(&self) -> &'static str {
        "RSpec/IdenticalEqualityAssertion"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<Correction>>,
    ) {
        if safe_send(source, node) {
            return;
        }
        if name(source, node) != Some(b"to") {
            return;
        }
        let Some(receiver) = call_receiver(node).filter(|recv| bare(source, *recv, b"expect"))
        else {
            return;
        };
        let actual = call_args(receiver);
        let expected = call_args(node);
        if actual.len() != 1 || expected.len() != 1 {
            return;
        }
        let matcher = expected[0];
        let values = call_args(matcher);
        let right = if call_receiver(matcher).is_none()
            && matches!(name(source, matcher), Some(b"eql" | b"eq" | b"be"))
            && values.len() == 1
        {
            values[0]
        } else if matcher.kind() == "binary"
            && matcher
                .child_by_field_name("operator")
                .is_some_and(|operator| node_bytes(source, operator) == b"==")
            && matcher
                .child_by_field_name("left")
                .is_some_and(|left| bare(source, left, b"be"))
        {
            matcher.child_by_field_name("right").unwrap()
        } else {
            return;
        };
        if !same_expression(source, actual[0], right) {
            return;
        }
        report(
            self,
            source,
            node.start_byte(),
            header_end(node),
            "Identical expressions on both sides of the equality may indicate a flawed test."
                .into(),
            diagnostics,
        );
    }
}
