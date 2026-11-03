use crate::cop::rspec::helpers::{
    RSPEC_INCLUDE, bare_rspec_call, call_block, is_example, is_group,
};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::rspec::separation::numbered_block;
use crate::cop::shared::{call_method_name, call_receiver, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct PendingWithoutReason;
#[derive(Clone, Copy)]
enum Parent<'a> {
    Block(Node<'a>),
    Begin(Node<'a>),
    Other,
}
fn parent(node: Node<'_>) -> Option<Parent<'_>> {
    let direct = node.parent()?;
    match direct.kind() {
        "body_statement" | "block_body" => {
            if code_children(direct).len() == 1 {
                parent(direct)
            } else {
                Some(Parent::Begin(direct))
            }
        }
        "block" | "do_block" => direct.parent().map(Parent::Block),
        "program" => (code_children(direct).len() > 1).then_some(Parent::Begin(direct)),
        "parenthesized_statements" => Some(Parent::Begin(direct)),
        _ => Some(Parent::Other),
    }
}
fn scope(node: Node<'_>) -> Option<Parent<'_>> {
    let first = parent(node)?;
    if let Parent::Begin(begin) = first {
        Some(parent(begin).unwrap_or(first))
    } else {
        Some(first)
    }
}
fn skipped_example(method: &[u8]) -> bool {
    matches!(
        method,
        b"skip" | b"xit" | b"xexample" | b"xspecify" | b"xscenario"
    )
}
fn metadata(source: &SourceFile, node: Node<'_>) -> Option<String> {
    let method = bare_rspec_call(source, node).or_else(|| {
        (node.kind() == "identifier")
            .then(|| name(source, node))
            .flatten()
    })?;
    if !(is_example(method) || is_example_group(method)) {
        return None;
    }
    let args = call_args(node);
    let last = *args.last()?;
    if symbol_node(last) {
        return crate::cop::ported::static_symbol(source, last)
            .filter(|value| matches!(value.as_str(), "skip" | "pending"));
    }
    let pairs: Vec<_> = if last.kind() == "hash" {
        code_children(last)
    } else if matches!(last.kind(), "pair" | "hash_splat_argument") {
        args.into_iter()
            .filter(|arg| arg.kind() == "pair")
            .collect()
    } else {
        return None;
    };
    for pair in pairs.into_iter().filter(|pair| pair.kind() == "pair") {
        let Some(key) = pair.child_by_field_name("key") else {
            continue;
        };
        if pair
            .child_by_field_name("value")
            .is_some_and(|value| value.kind() == "true")
        {
            if let Some(value) = crate::cop::ported::static_symbol(source, key)
                .filter(|value| matches!(value.as_str(), "skip" | "pending"))
            {
                return Some(value);
            }
        }
    }
    None
}
impl Cop for PendingWithoutReason {
    fn name(&self) -> &'static str {
        "RSpec/PendingWithoutReason"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "identifier"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        crate::cop::rspec::helpers::example_and_group_names()
    }
    fn interested_identifier_names(&self) -> &'static [&'static [u8]] {
        self.interested_call_names()
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        let method_name = if node.kind() == "identifier" {
            Some(node_bytes(source, node))
        } else {
            call_method_name(source, node)
        };
        if !method_name.is_some_and(|method| {
            crate::cop::rspec::helpers::is_group(method)
                || crate::cop::rspec::helpers::is_example(method)
        }) {
            return;
        }
        if safe_send(source, node) {
            return;
        }
        let mut reasons = Vec::new();
        if let Some(reason) = metadata(source, node) {
            reasons.push(reason);
        }
        if let Some(parent) = scope(node) {
            let group = matches!(parent,Parent::Block(parent) if bare_rspec_call(source,parent).is_some_and(is_group));
            let explicit_group = call_block(node).is_some()
                && call_receiver(node).is_some()
                && bare_rspec_call(source, node).is_some_and(is_example_group);
            let method = name(source, node).unwrap_or_default();
            let args = call_args(node);
            if group || explicit_group {
                if call_receiver(node).is_none()
                    && (skipped_example(method) || method == b"pending")
                    && (args.is_empty() || call_block(node).is_some())
                {
                    reasons.push(String::from_utf8_lossy(method).into_owned());
                }
                if bare_rspec_call(source, node).is_some_and(|method| {
                    matches!(method, b"xdescribe" | b"xcontext" | b"xfeature")
                }) {
                    reasons.push("skip".into());
                }
            } else if matches!(parent,Parent::Block(parent) if call_receiver(parent).is_none() && bare_rspec_call(source,parent).is_some_and(is_example) && !numbered_block(source,parent))
                && call_receiver(node).is_none()
                && (skipped_example(method) || method == b"pending")
                && args.is_empty()
            {
                reasons.push(String::from_utf8_lossy(method).into_owned());
            }
        }
        if let Some(reason) = reasons.into_iter().next() {
            report(
                self,
                source,
                node.start_byte(),
                header_end(node),
                format!("Give the reason for {reason}."),
                diagnostics,
            );
        }
    }
}
