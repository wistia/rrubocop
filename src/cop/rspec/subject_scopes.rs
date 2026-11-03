use super::helpers::{bare_rspec_call, call_block, is_example};
use super::parity_helpers::*;
use super::separation::numbered_block;
use crate::cop::shared::{call_receiver, node_bytes};
use crate::parse::source::SourceFile;
use std::collections::HashMap;
use tree_sitter::Node;

pub(super) type Names = HashMap<super::cache::NodeRange, Vec<Vec<u8>>>;
fn uncached_definitions(source: &SourceFile, top: Node<'_>, helpers: bool) -> Names {
    let mut names = Names::new();
    let mut pending = code_children(top);
    while let Some(node) = pending.pop() {
        pending.extend(code_children(node));
        if call_receiver(node).is_some()
            || safe_send(source, node)
            || call_block(node).is_none()
            || numbered_block(source, node)
        {
            continue;
        }
        let args = call_args(node);
        let Some(method) = name(source, node) else {
            continue;
        };
        if if helpers {
            method != b"let"
        } else {
            !matches!(method, b"subject" | b"subject!")
        } {
            continue;
        }
        let value = if args.is_empty() && !helpers {
            method.to_vec()
        } else if args.len() == 1 && symbol_node(args[0]) {
            let Some(value) = crate::cop::ported::static_symbol(source, args[0]) else {
                continue;
            };
            value.into_bytes()
        } else {
            continue;
        };
        let mut ancestor = node.parent();
        while let Some(group) = ancestor {
            if bare_rspec_call(source, group).is_some_and(is_example_group)
                && call_block(group).is_some()
                && !numbered_block(source, group)
            {
                names
                    .entry(super::cache::NodeRange::new(group))
                    .or_default()
                    .push(value);
                break;
            }
            ancestor = group.parent();
        }
    }
    names
}
#[derive(Clone)]
struct SubjectDefinitions(std::sync::Arc<Names>);
#[derive(Clone)]
struct HelperDefinitions(std::sync::Arc<Names>);
pub(super) fn definitions(
    source: &SourceFile,
    top: Node<'_>,
    helpers: bool,
) -> std::sync::Arc<Names> {
    if helpers {
        super::cache::node_fact(source, top, || {
            HelperDefinitions(std::sync::Arc::new(uncached_definitions(source, top, true)))
        })
        .0
    } else {
        super::cache::node_fact(source, top, || {
            SubjectDefinitions(std::sync::Arc::new(uncached_definitions(
                source, top, false,
            )))
        })
        .0
    }
}

pub(super) fn example(source: &SourceFile, node: Node<'_>) -> bool {
    call_receiver(node).is_none()
        && bare_rspec_call(source, node).is_some_and(is_example)
        && call_block(node).is_some()
        && !numbered_block(source, node)
}
fn send_node(source: &SourceFile, node: Node<'_>) -> bool {
    match node.kind() {
        "call" => !numbered_block(source, node),
        "identifier" => crate::cop::ported::bare_call_identifier(source, node),
        "binary" => node.child_by_field_name("operator").is_none_or(|operator| {
            !matches!(node_bytes(source, operator), b"&&" | b"||" | b"and" | b"or")
        }),
        "unary" | "element_reference" => true,
        _ => false,
    }
}
fn append<'a>(source: &SourceFile, node: Node<'a>, children: &mut Vec<Node<'a>>) {
    if matches!(
        node.kind(),
        "body_statement" | "block_body" | "argument_list"
    ) {
        if code_children(node)
            .iter()
            .any(|child| matches!(child.kind(), "rescue" | "ensure"))
        {
            return;
        }
        for child in code_children(node) {
            append(source, child, children);
        }
    } else if send_node(source, node)
        || matches!(node.kind(), "method" | "parenthesized_statements")
    {
        children.push(node);
    }
}
pub(super) fn body_children<'a>(source: &SourceFile, node: Node<'a>) -> Vec<Node<'a>> {
    let mut children = Vec::new();
    if let Some(body) = call_block(node).and_then(|block| block.child_by_field_name("body")) {
        append(source, body, &mut children);
    }
    children
}
pub(super) fn children<'a>(source: &SourceFile, node: Node<'a>) -> Vec<Node<'a>> {
    let mut children = Vec::new();
    if node.kind() == "call" {
        if let Some(receiver) = call_receiver(node) {
            append(source, receiver, &mut children);
        }
        for argument in call_args(node) {
            append(source, argument, &mut children);
        }
        children.extend(body_children(source, node));
    } else if node.kind() == "method" {
        if let Some(body) = node.child_by_field_name("body") {
            append(source, body, &mut children);
        }
    } else {
        for child in code_children(node) {
            append(source, child, &mut children);
        }
    }
    children
}
pub(super) fn parent_send(source: &SourceFile, node: Node<'_>) -> bool {
    if call_block(node).is_some() {
        return false;
    }
    let Some(parent) = node.parent() else {
        return false;
    };
    let parent = if parent.kind() == "argument_list" {
        parent.parent().unwrap_or(parent)
    } else {
        parent
    };
    send_node(source, parent)
}
