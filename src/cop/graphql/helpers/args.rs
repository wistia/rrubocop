//! Call / symbol / positional-arg helpers for GraphQL Ruby DSL cops.

use tree_sitter::Node;

use crate::cop::shared::{argument_nodes, call_method_name, call_receiver, node_text};
use crate::parse::source::SourceFile;

pub const CALL_KINDS: &[&str] = &["call", "command", "command_call"];

pub const DEPT_INCLUDE: &[&str] = &["**/graphql/**/*"];

/// Bare `method` call/command with no receiver.
pub fn bare_method(source: &SourceFile, node: Node<'_>, name: &[u8]) -> bool {
    if !matches!(node.kind(), "call" | "command" | "command_call") {
        return false;
    }
    if call_receiver(node).is_some() {
        return false;
    }
    call_method_name(source, node) == Some(name)
}

pub fn is_field_call(source: &SourceFile, node: Node<'_>) -> bool {
    bare_method(source, node, b"field") && first_sym_arg(source, node).is_some()
}

pub fn is_argument_call(source: &SourceFile, node: Node<'_>) -> bool {
    (bare_method(source, node, b"argument") || bare_method(source, node, b"public_argument"))
        && first_sym_arg(source, node).is_some()
}

/// Symbol text without leading `:`.
pub fn sym_text(source: &SourceFile, node: Node<'_>) -> Option<String> {
    let t = node_text(source, node);
    match node.kind() {
        "simple_symbol" | "hash_key_symbol" | "symbol" => {
            Some(t.trim_start_matches(':').to_string())
        }
        _ => None,
    }
}

pub fn first_sym_arg(source: &SourceFile, node: Node<'_>) -> Option<String> {
    let first = argument_nodes(node)
        .into_iter()
        .find(|n| n.kind() != "comment")?;
    sym_text(source, first).or_else(|| {
        if matches!(first.kind(), "string" | "string_content") {
            let mut cursor = first.walk();
            if first
                .named_children(&mut cursor)
                .any(|n| n.kind() == "interpolation")
            {
                return None;
            }
            Some(strip_quotes(&node_text(source, first)))
        } else {
            None
        }
    })
}

/// Plain `field :name` send — excludes block/resolver bodies (`field :x do`).
pub fn plain_field_definition(source: &SourceFile, node: Node<'_>) -> bool {
    is_field_call(source, node) && call_block(node).is_none()
}

fn strip_quotes(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

/// Positional (non-pair) arguments of a call.
pub fn positional_args<'a>(node: Node<'a>) -> Vec<Node<'a>> {
    argument_nodes(node)
        .into_iter()
        .filter(|n| n.kind() != "pair" && n.kind() != "hash")
        .collect()
}

pub fn call_block(node: Node<'_>) -> Option<Node<'_>> {
    node.child_by_field_name("block")
}
