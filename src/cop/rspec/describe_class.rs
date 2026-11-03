use crate::cop::rspec::helpers::{RSPEC_INCLUDE, bare_rspec_call};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct DescribeClass;
impl Cop for DescribeClass {
    fn name(&self) -> &'static str {
        "RSpec/DescribeClass"
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
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<Correction>>,
    ) {
        if safe_send(source, node) {
            return;
        }
        if bare_rspec_call(source, node) != Some(b"describe")
            || !top_groups(source, node).contains(&node)
        {
            return;
        }
        let args = call_args(node);
        let Some(first) = args.first().copied() else {
            return;
        };
        if matches!(first.kind(), "constant" | "scope_resolution") {
            return;
        }
        if first.kind() == "string"
            && crate::cop::ported::static_literal(source, first)
                .is_some_and(|value| constant_description(&value))
        {
            return;
        }
        if ignored_metadata(source, node, config) {
            return;
        }
        report(
            self,
            source,
            first.start_byte(),
            first.end_byte(),
            "The first argument to describe should be the class or module being tested.".into(),
            diagnostics,
        );
    }
}
