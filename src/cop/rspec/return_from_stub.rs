use crate::cop::rspec::helpers::{RSPEC_INCLUDE, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::rspec::separation::numbered_block;
use crate::cop::shared::{call_receiver, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

fn contains_stub(source: &SourceFile, node: Node<'_>) -> bool {
    let mut pending = vec![node];
    while let Some(child) = pending.pop() {
        if bare(source, child, b"receive")
            && !safe_send(source, child)
            && call_args(child)
                .iter()
                .position(|arg| matches!(arg.kind(), "pair" | "hash_splat_argument"))
                .map_or(call_args(child).len(), |index| index + 1)
                == 1
        {
            return true;
        }
        if child == node {
            pending.extend(call_receiver(child));
            pending.extend(call_args(child));
        } else {
            pending.extend(code_children(child));
        }
    }
    false
}

pub struct ReturnFromStub;
impl Cop for ReturnFromStub {
    fn name(&self) -> &'static str {
        "RSpec/ReturnFromStub"
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
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if !contains_stub(source, node) {
            return;
        }
        if config.get_str("EnforcedStyle", "and_return") == "block" {
            if name(source, node) != Some(b"and_return") || safe_send(source, node) {
                return;
            }
            let args = call_args(node);
            let implicit_hash = !args.is_empty()
                && args
                    .iter()
                    .all(|arg| matches!(arg.kind(), "pair" | "hash_splat_argument"));
            if args.is_empty()
                || (!implicit_hash && args.len() != 1)
                || args.iter().any(|arg| !literal_or_const(source, *arg))
            {
                return;
            }
            let target = selector(node);
            let index = report(
                self,
                source,
                target.start_byte(),
                target.end_byte(),
                "Use block for static values.".into(),
                diagnostics,
            );
            if args.len() == 1 && args[0].kind() == "heredoc_beginning" {
                diagnostics[index].correctable = false;
            }
            if call_block(node).is_some()
                || implicit_hash
                || !stable_immutable_value(source, args[0])
            {
                return;
            }
            let Some(receiver) = call_receiver(node) else {
                return;
            };
            let replacement = format!(
                " {{ {} }}",
                String::from_utf8_lossy(node_bytes(source, args[0]))
            );
            if preserving_replace(
                source,
                node,
                &mut corrections,
                receiver.end_byte(),
                node.end_byte(),
                replacement,
                self.name(),
            ) {
                diagnostics[index].corrected = true;
            }
        } else {
            if numbered_block(source, node) {
                return;
            }
            let Some(block) = call_block(node) else {
                return;
            };
            let statements = block
                .child_by_field_name("body")
                .map(code_children)
                .unwrap_or_default()
                .into_iter()
                .filter(|statement| statement.kind() != "heredoc_body")
                .collect::<Vec<_>>();
            if statements
                .iter()
                .any(|body| !literal_or_const(source, *body))
            {
                return;
            }
            let start = block.start_byte();
            let token = if node_bytes(source, block).starts_with(b"do") {
                2
            } else {
                1
            };
            let index = report(
                self,
                source,
                start,
                start + token,
                "Use `and_return` for static values.".into(),
                diagnostics,
            );
            if statements.len() == 1 && statements[0].kind() == "heredoc_beginning" {
                diagnostics[index].correctable = false;
            }
            if block
                .child_by_field_name("parameters")
                .is_some_and(|parameters| {
                    code_children(parameters)
                        .iter()
                        .any(|parameter| parameter.kind() != "identifier")
                })
                || statements.len() > 1
                || statements
                    .first()
                    .is_some_and(|value| !stable_immutable_value(source, *value))
            {
                return;
            }
            let body = statements
                .first()
                .map(|body| String::from_utf8_lossy(node_bytes(source, *body)).into_owned())
                .unwrap_or_else(|| "nil".into());
            let header_end = header_end(node);
            let replacement = format!(
                "{}.and_return({body})",
                String::from_utf8_lossy(&source.as_bytes()[node.start_byte()..header_end])
            );
            if preserving_replace(
                source,
                node,
                &mut corrections,
                node.start_byte(),
                node.end_byte(),
                replacement,
                self.name(),
            ) {
                diagnostics[index].corrected = true;
            }
        }
    }
}
