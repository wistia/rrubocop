use crate::cop::rspec::helpers::{RSPEC_INCLUDE, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::rspec::separation::numbered_block;
use crate::cop::shared::{call_method_name, call_receiver, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

fn constant_type(node: Node<'_>) -> bool {
    matches!(node.kind(), "constant" | "scope_resolution")
}
fn constant_name(source: &SourceFile, node: Node<'_>) -> Option<String> {
    if node.kind() == "constant" {
        return Some(String::from_utf8_lossy(node_bytes(source, node)).into());
    }
    if node.kind() != "scope_resolution" {
        return None;
    }
    let name = node.child_by_field_name("name")?;
    if name.kind() != "constant" {
        return None;
    }
    let name = String::from_utf8_lossy(node_bytes(source, name));
    match node.child_by_field_name("scope") {
        Some(scope) => Some(format!("{}::{name}", constant_name(source, scope)?)),
        None => Some(name.into_owned()),
    }
}
fn stable_receiver(source: &SourceFile, node: Node<'_>, message: &str) -> bool {
    constant_name(source, node).is_some()
        || node.kind() == "self"
        || stable_immutable_value(source, node)
        || matches!(message, "size" | "length" | "count")
            && matches!(node.kind(), "array" | "hash" | "string" | "character")
            && pure_literal(source, node)
}
fn getter_valid(receiver: &str, message: &str) -> bool {
    if receiver.starts_with(['+', '-']) {
        return false;
    }
    let source = SourceFile::from_bytes(
        "getter.rb",
        format!("[({receiver}), {receiver}.{message}]").into_bytes(),
    );
    let Ok(tree) = crate::parse::parse_ruby(&source) else {
        return false;
    };
    if tree.root_node().has_error() {
        return false;
    }
    let Some(array) = tree.root_node().named_child(0) else {
        return false;
    };
    let values = code_children(array);
    if values.len() != 2 {
        return false;
    }
    let original = if values[0].kind() == "parenthesized_statements" {
        code_children(values[0]).first().copied()
    } else {
        Some(values[0])
    };
    values[1].kind() == "call"
        && call_method_name(&source, values[1]) == Some(message.as_bytes())
        && call_args(values[1]).is_empty()
        && call_block(values[1]).is_none()
        && original
            .zip(call_receiver(values[1]))
            .is_some_and(|(old, new)| {
                crate::cop::shared::ruby_expression::eql_expression(&source, old, new)
            })
}
fn constant_mutation(source: &SourceFile, node: Node<'_>, constant: &str) -> bool {
    let mut ancestor = node.parent();
    let mut event = None;
    while let Some(parent) = ancestor {
        if runner(source, parent) {
            event = call_receiver(parent)
                .filter(|target| bare(source, *target, b"expect"))
                .and_then(call_block);
            if event.is_some() {
                break;
            }
        }
        ancestor = parent.parent();
    }
    let Some(event) = event else {
        return false;
    };
    let mut pending = vec![event];
    while let Some(child) = pending.pop() {
        if matches!(
            name(source, child),
            Some(b"stub_const" | b"hide_const" | b"const_set" | b"remove_const")
        ) {
            let args = call_args(child);
            let target = args
                .first()
                .and_then(|target| crate::cop::ported::static_literal(source, *target));
            if target.is_none_or(|target| {
                let target = target.trim_start_matches("::");
                constant == target
                    || constant.starts_with(&format!("{target}::"))
                    || constant.rsplit("::").next() == Some(target)
            }) {
                return true;
            }
        }
        if child.kind() == "assignment"
            && child
                .child_by_field_name("left")
                .and_then(|left| constant_name(source, left))
                .is_some_and(|target| {
                    constant == target || constant.starts_with(&format!("{target}::"))
                })
        {
            return true;
        }
        pending.extend(code_children(child));
    }
    false
}
fn message(source: &SourceFile, node: Node<'_>) -> Option<String> {
    if symbol_node(node) {
        return crate::cop::ported::static_symbol(source, node).filter(|_| {
            crate::cop::ported::ruby_string_parts(source, node).is_none_or(|parts| parts.len() == 1)
        });
    }
    matches!(node.kind(), "string" | "character" | "heredoc_beginning")
        .then(|| crate::cop::ported::ruby_string_parts(source, node))
        .flatten()
        .filter(|parts| parts.len() == 1)
        .map(|parts| parts.concat())
}
pub struct ExpectChange;
impl Cop for ExpectChange {
    fn name(&self) -> &'static str {
        "RSpec/ExpectChange"
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
    fn safe_autocorrect(&self) -> bool {
        false
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if !bare(source, node, b"change") || safe_send(source, node) {
            return;
        }
        if config.get_str("EnforcedStyle", "method_call") == "block" {
            let args = call_args(node);
            if args.len() != 2 {
                return;
            }
            let Some(message) = message(source, args[1]) else {
                return;
            };
            let receiver = String::from_utf8_lossy(node_bytes(source, args[0]));
            let index = report(
                self,
                source,
                node.start_byte(),
                header_end(node),
                format!("Prefer `change {{ {receiver}.{message} }}`."),
                diagnostics,
            );
            if corrections.is_none() {
                return;
            }
            if call_block(node).is_some()
                || !stable_receiver(source, args[0], &message)
                || !getter_valid(&receiver, &message)
                || constant_name(source, args[0])
                    .is_some_and(|constant| constant_mutation(source, node, &constant))
            {
                return;
            }
            let replacement = format!("change {{ {receiver}.{message} }}");
            if preserving_replace(
                source,
                node,
                &mut corrections,
                node.start_byte(),
                header_end(node),
                replacement,
                self.name(),
            ) {
                diagnostics[index].corrected = true;
            }
        } else {
            if !call_args(node).is_empty() || numbered_block(source, node) {
                return;
            }
            let Some(block) = call_block(node) else {
                return;
            };
            if block
                .child_by_field_name("parameters")
                .is_some_and(|parameters| !code_children(parameters).is_empty())
            {
                return;
            }
            let body = block
                .child_by_field_name("body")
                .map(code_children)
                .unwrap_or_default();
            if body.len() != 1
                || body[0].kind() != "call"
                || safe_send(source, body[0])
                || !call_args(body[0]).is_empty()
                || call_block(body[0]).is_some()
            {
                return;
            }
            let Some(receiver) = call_receiver(body[0]) else {
                return;
            };
            let method_receiver = !safe_send(source, receiver)
                && call_receiver(receiver).is_none()
                && name(source, receiver).is_some()
                && call_args(receiver).is_empty()
                && call_block(receiver).is_none();
            if !constant_type(receiver) && !method_receiver {
                return;
            }
            let Some(message) = name(source, body[0]) else {
                return;
            };
            let object = String::from_utf8_lossy(node_bytes(source, receiver));
            let method = String::from_utf8_lossy(message);
            let index = report(
                self,
                source,
                node.start_byte(),
                node.end_byte(),
                format!("Prefer `change({object}, :{method})`."),
                diagnostics,
            );
            if corrections.is_none() {
                return;
            }
            let Some(constant) = constant_name(source, receiver) else {
                return;
            };
            if constant_mutation(source, node, &constant) {
                return;
            }
            let replacement = format!("change({object}, :{method})");
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
