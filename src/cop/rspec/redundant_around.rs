use crate::cop::rspec::helpers::{RSPEC_INCLUDE, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::rspec::separation::numbered_block;
use crate::cop::shared::{call_receiver, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct RedundantAround;
impl Cop for RedundantAround {
    fn name(&self) -> &'static str {
        "RSpec/RedundantAround"
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
        if name(source, node) != Some(b"around") || safe_send(source, node) {
            return;
        }
        let args = call_args(node);
        let block = call_block(node);
        let block_pass = args
            .last()
            .filter(|argument| argument.kind() == "block_argument")
            .and_then(|argument| code_children(*argument).first().copied())
            .is_some_and(|value| {
                symbol_node(value)
                    && crate::cop::ported::static_symbol(source, value).as_deref() == Some("run")
            });
        let body = block.and_then(|block| block.child_by_field_name("body"));
        let statements = body.map(code_children).unwrap_or_default();
        let run = statements
            .first()
            .copied()
            .filter(|_| statements.len() == 1)
            .filter(|run| {
                name(source, *run) == Some(b"run")
                    && !safe_send(source, *run)
                    && call_args(*run).is_empty()
                    && call_block(*run).is_none()
            });
        if !block_pass && run.is_none() {
            return;
        }
        let index = report(
            self,
            source,
            node.start_byte(),
            node.end_byte(),
            "Remove redundant `around` hook.".into(),
            diagnostics,
        );
        let proxy_run = run.is_some_and(|run| {
            let Some(receiver) = call_receiver(run) else {
                return false;
            };
            if receiver.kind() != "identifier" {
                return false;
            }
            if numbered_block(source, node) {
                return node_bytes(source, receiver) == b"_1";
            }
            let parameters = block
                .and_then(|block| block.child_by_field_name("parameters"))
                .map(code_children)
                .unwrap_or_default();
            parameters.len() == 1
                && parameters[0].kind() == "identifier"
                && node_bytes(source, parameters[0]) == node_bytes(source, receiver)
                && crate::cop::ported::local_identifier(source, receiver)
        });
        if call_receiver(node).is_none()
            && args
                .iter()
                .all(|arg| symbol_node(*arg) || arg.kind() == "block_argument")
            && (block_pass || proxy_run)
            && preserving_replace(
                source,
                node,
                &mut corrections,
                node.start_byte(),
                node.end_byte(),
                "",
                self.name(),
            )
        {
            diagnostics[index].corrected = true;
        }
    }
}
