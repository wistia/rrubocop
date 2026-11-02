//! Shared helpers for RSpec cops (tree-sitter).

use tree_sitter::Node;

use crate::cop::shared::{call_method_name, call_receiver, node_bytes};
use crate::parse::source::SourceFile;

pub const RSPEC_INCLUDE: &[&str] = &["**/*_spec.rb", "**/spec/**/*"];

const EXAMPLES: &[&[u8]] = &[
    b"it",
    b"specify",
    b"example",
    b"its",
    b"focus",
    b"fexample",
    b"fit",
    b"fspecify",
    b"skip",
    b"xexample",
    b"xit",
    b"xspecify",
    b"pending",
    b"scenario",
    b"fscenario",
    b"xscenario",
];

const GROUPS: &[&[u8]] = &[
    b"describe",
    b"context",
    b"feature",
    b"example_group",
    b"fdescribe",
    b"fcontext",
    b"xdescribe",
    b"xcontext",
    b"xfeature",
    b"ffeature",
    b"shared_examples",
    b"shared_context",
    b"shared_examples_for",
];

const LETS: &[&[u8]] = &[b"let", b"let!", b"subject", b"subject!"];

pub fn is_example(name: &[u8]) -> bool {
    EXAMPLES.iter().any(|&e| e == name)
}

pub fn is_group(name: &[u8]) -> bool {
    GROUPS.iter().any(|&e| e == name)
}

pub fn is_let_helper(name: &[u8]) -> bool {
    LETS.iter().any(|&e| e == name)
}

pub fn bare_rspec_call<'a>(source: &'a SourceFile, node: Node<'_>) -> Option<&'a [u8]> {
    if node
        .child_by_field_name("operator")
        .is_some_and(|operator| node_bytes(source, operator) == b"&.")
    {
        return None;
    }
    let name = call_method_name(source, node)?;
    if let Some(recv) = call_receiver(node) {
        if !((recv.kind() == "constant" && node_bytes(source, recv) == b"RSpec")
            || (recv.kind() == "scope_resolution" && node_bytes(source, recv) == b"::RSpec"))
        {
            return None;
        }
    }
    Some(name)
}

pub fn call_block(node: Node<'_>) -> Option<Node<'_>> {
    let mut cur = node.walk();
    node.children(&mut cur)
        .find(|c| matches!(c.kind(), "do_block" | "block"))
}

pub fn block_body(block: Node<'_>) -> Option<Node<'_>> {
    block.child_by_field_name("body")
}

fn line_is_code(bytes: &[u8], line_no: usize, source: &SourceFile) -> bool {
    let Some(ls) = source.line_start(line_no) else {
        return false;
    };
    let trimmed = trim_ws(&bytes[ls..source.line_start(line_no + 1).unwrap_or(bytes.len())]);
    !trimmed.is_empty() && trimmed[0] != b'#'
}

/// Count non-blank, non-comment source lines covered by `node`.
pub fn code_lines(source: &SourceFile, node: Node<'_>) -> usize {
    let bytes = source.as_bytes();
    let (start, end) = (node.start_byte(), node.end_byte());
    if start >= end || end > bytes.len() {
        return 0;
    }
    let (lo, _) = source.offset_to_line_col(start);
    let (hi, _) = source.offset_to_line_col(end.saturating_sub(1));
    (lo..=hi)
        .filter(|&n| line_is_code(bytes, n, source))
        .count()
}

fn trim_ws(line: &[u8]) -> &[u8] {
    let s = line
        .iter()
        .position(|&b| !matches!(b, b' ' | b'\t' | b'\r'))
        .unwrap_or(line.len());
    &line[s..line
        .iter()
        .rposition(|&b| !matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
        .map_or(s, |i| i + 1)]
}

fn unquote(b: &[u8]) -> &[u8] {
    let inner = b
        .strip_prefix(b"'")
        .or_else(|| b.strip_prefix(b"\""))
        .unwrap_or(b);
    inner
        .strip_suffix(b"'")
        .or_else(|| inner.strip_suffix(b"\""))
        .unwrap_or(inner)
}

/// First symbol / string arg of a call (let(:name), subject(:name), …).
pub fn first_sym_arg<'a>(source: &'a SourceFile, node: Node<'_>) -> Option<&'a [u8]> {
    let first = crate::cop::shared::argument_nodes(node)
        .into_iter()
        .next()?;
    match first.kind() {
        "simple_symbol" | "symbol" => {
            let b = node_bytes(source, first);
            Some(b.strip_prefix(b":").unwrap_or(b))
        }
        "string" => Some(unquote(node_bytes(source, first))),
        _ => None,
    }
}

fn is_spec_group_call(source: &SourceFile, node: Node<'_>) -> bool {
    bare_rspec_call(source, node).is_some_and(is_group) && call_block(node).is_some()
}

/// RSpec group ancestry through an ordinary block, including its send arguments.
pub fn inside_spec_group(source: &SourceFile, node: Node<'_>) -> bool {
    let mut ancestor = node.parent();
    while let Some(parent) = ancestor {
        if is_spec_group_call(source, parent) && !super::separation::numbered_block(source, parent)
        {
            return true;
        }
        ancestor = parent.parent();
    }
    false
}

fn named_code_children(node: Node<'_>) -> impl Iterator<Item = Node<'_>> {
    (0..node.named_child_count() as u32)
        .filter_map(move |i| node.named_child(i).filter(|c| c.kind() != "comment"))
}

fn collect_top_level_groups<'a>(source: &SourceFile, node: Node<'a>) -> Vec<Node<'a>> {
    match node.kind() {
        "module" | "class" => node
            .child_by_field_name("body")
            .into_iter()
            .flat_map(named_code_children)
            .flat_map(|child| collect_top_level_groups(source, child))
            .collect(),
        "call" | "command" if is_spec_group_call(source, node) => vec![node],
        _ => Vec::new(),
    }
}

pub(super) fn top_level_groups<'a>(source: &SourceFile, program: Node<'a>) -> Vec<Node<'a>> {
    let stmts: Vec<_> = named_code_children(program).collect();
    match stmts.as_slice() {
        [only] => collect_top_level_groups(source, *only),
        _ => stmts
            .into_iter()
            .filter(|s| is_spec_group_call(source, *s))
            .collect(),
    }
}

/// RuboCop `TopLevelGroup`: example/shared groups at the file root.
/// A sole top-level module/class is unwrapped; sibling statements are not.
pub fn node_in_top_level_group(source: &SourceFile, node: Node<'_>) -> bool {
    let mut p = node.parent();
    while let Some(cur) = p {
        if cur.kind() == "program" {
            return top_level_groups(source, cur)
                .iter()
                .any(|g| node.start_byte() >= g.start_byte() && node.end_byte() <= g.end_byte());
        }
        p = cur.parent();
    }
    false
}
