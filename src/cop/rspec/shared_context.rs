use crate::cop::rspec::helpers::{
    RSPEC_INCLUDE, bare_rspec_call, call_block, is_example, is_let_helper,
};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::rspec::separation::numbered_block;
use crate::cop::shared::call_receiver;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct SharedContext;
impl Cop for SharedContext {
    fn name(&self) -> &'static str {
        "RSpec/SharedContext"
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
        let _ = config;
        if safe_send(source, node) || call_block(node).is_none() || numbered_block(source, node) {
            return;
        }
        let Some(method) = bare_rspec_call(source, node) else {
            return;
        };
        if !matches!(
            method,
            b"shared_context" | b"shared_examples" | b"shared_examples_for"
        ) {
            return;
        }
        let mut examples = false;
        let mut context = false;
        let mut pending = code_children(node);
        while let Some(child) = pending.pop() {
            if !safe_send(source, child) && call_receiver(child).is_none() {
                if let Some(method) = name(source, child) {
                    examples |= is_example(method)
                        || matches!(
                            method,
                            b"include_examples" | b"it_behaves_like" | b"it_should_behave_like"
                        );
                    context |= is_let_helper(method)
                        || hooks().contains(&method)
                        || method == b"include_context";
                }
            }
            pending.extend(code_children(child));
        }
        let replacement = if method == b"shared_context" && examples && !context {
            "shared_examples"
        } else if method != b"shared_context" && context && !examples {
            "shared_context"
        } else {
            return;
        };
        let message = if replacement == "shared_examples" {
            "Use `shared_examples` when you don't define context."
        } else {
            "Use `shared_context` when you don't define examples."
        };
        let index = report(
            self,
            source,
            node.start_byte(),
            header_end(node),
            message.into(),
            diagnostics,
        );
        let target = selector(node);
        if preserving_replace(
            source,
            node,
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
