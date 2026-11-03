use crate::cop::rspec::helpers::{RSPEC_INCLUDE, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::rspec::separation::numbered_block;
use crate::cop::shared::{call_receiver, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

fn receives(source: &SourceFile, node: Node<'_>) -> bool {
    let mut pending = call_args(node);
    pending.extend(call_receiver(node));
    if bare(source, node, b"receive") && !safe_send(source, node) {
        return true;
    }
    while let Some(child) = pending.pop() {
        if bare(source, child, b"receive") && !safe_send(source, child) {
            return true;
        }
        pending.extend(code_children(child));
    }
    false
}

pub struct Yield;
impl Cop for Yield {
    fn name(&self) -> &'static str {
        "RSpec/Yield"
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
        if numbered_block(source, node) || !receives(source, node) {
            return;
        }
        let Some(block) = call_block(node) else {
            return;
        };
        let parameters = block
            .child_by_field_name("parameters")
            .map(code_children)
            .unwrap_or_default();
        if parameters.len() != 1 || parameters[0].kind() != "block_parameter" {
            return;
        }
        let Some(variable) = parameters[0].child_by_field_name("name") else {
            return;
        };
        let Some(body) = block.child_by_field_name("body") else {
            return;
        };
        let mut calls = code_children(body);
        let parenthesized = calls.len() == 1 && calls[0].kind() == "parenthesized_statements";
        if parenthesized {
            calls = code_children(calls[0]);
        }
        if calls.is_empty() && !parenthesized
            || calls.iter().any(|call| {
                safe_send(source, *call)
                    || name(source, *call) != Some(b"call")
                    || call_block(*call).is_some()
                    || !call_receiver(*call).is_some_and(|receiver| {
                        receiver.kind() == "identifier"
                            && node_bytes(source, receiver) == node_bytes(source, variable)
                            && crate::cop::ported::local_identifier(source, receiver)
                    })
            })
        {
            return;
        }
        let index = report(
            self,
            source,
            block.start_byte(),
            block.end_byte(),
            "Use `.and_yield`.".into(),
            diagnostics,
        );
        if calls
            .iter()
            .flat_map(|call| call_args(*call))
            .any(|arg| !stable_immutable_value(source, arg))
        {
            return;
        }
        let mut replacement = String::new();
        for call in calls {
            replacement.push_str(".and_yield");
            let args = call_args(call);
            if !args.is_empty() {
                replacement.push('(');
                replacement.push_str(
                    &args
                        .iter()
                        .map(|arg| String::from_utf8_lossy(node_bytes(source, *arg)))
                        .collect::<Vec<_>>()
                        .join(", "),
                );
                replacement.push(')');
            }
        }
        let bytes = source.as_bytes();
        let mut start = block.start_byte();
        while start > node.start_byte() && matches!(bytes[start - 1], b' ' | b'\t') {
            start -= 1;
        }
        if preserving_replace(
            source,
            node,
            &mut corrections,
            start,
            block.end_byte(),
            replacement,
            self.name(),
        ) {
            diagnostics[index].corrected = true;
        }
    }
}
