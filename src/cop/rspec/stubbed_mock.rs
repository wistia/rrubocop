use crate::cop::rspec::helpers::{RSPEC_INCLUDE, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::rspec::separation::numbered_block;
use crate::cop::shared::call_receiver;
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

fn message_expectation(source: &SourceFile, node: Node<'_>) -> bool {
    if safe_send(source, node) || call_block(node).is_some() {
        return false;
    }
    if call_receiver(node).is_none()
        && matches!(
            name(source, node),
            Some(b"receive" | b"receive_message_chain")
        )
    {
        return true;
    }
    name(source, node) == Some(b"with")
        && call_receiver(node).is_some_and(|receiver| {
            bare(source, receiver, b"receive")
                && !safe_send(source, receiver)
                && call_block(receiver).is_none()
        })
}
fn response(source: &SourceFile, node: Node<'_>) -> bool {
    if safe_send(source, node) {
        return false;
    }
    if let Some(block) = call_block(node) {
        let args = block
            .child_by_field_name("parameters")
            .map(code_children)
            .unwrap_or_default();
        if args.is_empty() && !numbered_block(source, node) {
            if call_receiver(node).is_none()
                && matches!(
                    name(source, node),
                    Some(b"receive" | b"receive_message_chain")
                )
            {
                return true;
            }
            if name(source, node) == Some(b"with")
                && call_receiver(node).is_some_and(|receiver| {
                    bare(source, receiver, b"receive") && call_block(receiver).is_none()
                })
            {
                return true;
            }
        }
        return false;
    }
    let args = call_args(node);
    if let Some(receiver) = call_receiver(node) {
        if message_expectation(source, receiver)
            && matches!(
                name(source, node),
                Some(
                    b"and_return"
                        | b"and_raise"
                        | b"and_throw"
                        | b"and_yield"
                        | b"and_call_original"
                        | b"and_wrap_original"
                )
            )
            && ruby_arity(&args) == 1
        {
            return true;
        }
    }
    if call_receiver(node).is_none()
        && (name(source, node) == Some(b"receive_messages") && ruby_arity(&args) == 1
            || name(source, node) == Some(b"receive_message_chain") && !args.is_empty())
    {
        if args
            .last()
            .is_some_and(|last| matches!(last.kind(), "hash" | "pair" | "hash_splat_argument"))
        {
            return true;
        }
    }
    message_expectation(source, node)
        && args
            .last()
            .is_some_and(|last| last.kind() == "block_argument")
}

pub struct StubbedMock;
impl Cop for StubbedMock {
    fn name(&self) -> &'static str {
        "RSpec/StubbedMock"
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
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let _ = config;
        if !runner(source, node) || name(source, node) != Some(b"to") {
            return;
        }
        let Some(expectation) = call_receiver(node) else {
            return;
        };
        if safe_send(source, expectation)
            || call_receiver(expectation).is_some()
            || call_block(expectation).is_some()
            || !name(source, expectation).is_some_and(|name| expectations().contains(&name))
        {
            return;
        }
        let args = call_args(node);
        if args.len() != 1 {
            return;
        }
        let matcher = args[0];
        if !response(source, matcher) {
            return;
        }
        let method = name(source, expectation).unwrap();
        let replacement = match method {
            b"expect" => "allow",
            b"is_expected" => "allow(subject)",
            b"expect_any_instance_of" => "allow_any_instance_of",
            _ => "",
        };
        report(
            self,
            source,
            expectation.start_byte(),
            header_end(expectation),
            format!(
                "Prefer `{replacement}` over `{}` when configuring a response.",
                String::from_utf8_lossy(method)
            ),
            diagnostics,
        );
    }
}

fn ruby_arity(args: &[Node<'_>]) -> usize {
    args.iter()
        .position(|arg| matches!(arg.kind(), "pair" | "hash_splat_argument"))
        .map_or(args.len(), |index| index + 1)
}
