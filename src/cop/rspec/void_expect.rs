use crate::cop::rspec::helpers::{RSPEC_INCLUDE, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct VoidExpect;
impl Cop for VoidExpect {
    fn name(&self) -> &'static str {
        "RSpec/VoidExpect"
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
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<Correction>>,
    ) {
        if safe_send(source, node) {
            return;
        }
        if !bare(source, node, b"expect") {
            return;
        }
        if let Some(block) = call_block(node) {
            if block
                .child_by_field_name("parameters")
                .is_some_and(|params| params.named_child_count() > 0)
            {
                return;
            }
        }
        let Some(parent) = node.parent() else {
            return;
        };
        let is_void = if parent.kind() == "program" {
            true
        } else if matches!(parent.kind(), "body_statement" | "block_body") {
            code_children(parent).len() > 1
                || parent
                    .parent()
                    .is_some_and(|owner| matches!(owner.kind(), "block" | "do_block"))
        } else {
            false
        };
        if !is_void {
            return;
        }
        report(
            self,
            source,
            node.start_byte(),
            node.end_byte(),
            "Do not use `expect()` without `.to` or `.not_to`. Chain the methods or remove it."
                .into(),
            diagnostics,
        );
    }
}
