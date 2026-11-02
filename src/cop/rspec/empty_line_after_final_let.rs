use crate::cop::rspec::helpers::{RSPEC_INCLUDE, bare_rspec_call, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::rspec::separation::{numbered_block, separate};
use crate::cop::shared::call_receiver;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct EmptyLineAfterFinalLet;
impl Cop for EmptyLineAfterFinalLet {
    fn name(&self) -> &'static str {
        "RSpec/EmptyLineAfterFinalLet"
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
        if !bare_rspec_call(source, node).is_some_and(is_example_group)
            || numbered_block(source, node)
        {
            return;
        }
        let Some(body) = call_block(node).and_then(|block| block.child_by_field_name("body"))
        else {
            return;
        };
        let Some(last) = code_children(body).into_iter().rev().find(|child| {
            call_receiver(*child).is_none()
                && matches!(name(source, *child), Some(b"let" | b"let!"))
                && (call_block(*child).is_some()
                    || call_args(*child)
                        .iter()
                        .any(|arg| arg.kind() == "block_argument"))
        }) else {
            return;
        };
        separate(
            self,
            source,
            last,
            format!(
                "Add an empty line after the last `{}`.",
                String::from_utf8_lossy(name(source, last).unwrap())
            ),
            diagnostics,
            &mut corrections,
        );
    }
}
