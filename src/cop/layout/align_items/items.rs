//! Argument / hash item collection for alignment cops.

use tree_sitter::Node;

use crate::parse::source::SourceFile;

fn collect_items<'a>(node: Node<'a>) -> Vec<Node<'a>> {
    let mut cur = node.walk();
    node.named_children(&mut cur)
        .filter(|n| {
            !matches!(
                n.kind(),
                "comment" | "hash_splat_argument" | "forward_argument"
            )
        })
        .collect()
}

fn is_bare_hash(source: &SourceFile, node: Node<'_>) -> bool {
    matches!(node.kind(), "hash" | "bare_assoc_hash" | "bare_hash")
        && source
            .as_bytes()
            .get(node.start_byte())
            .is_some_and(|&b| b != b'{')
}

fn hash_pairs<'a>(hash: Node<'a>) -> Vec<Node<'a>> {
    let mut cur = hash.walk();
    hash.named_children(&mut cur)
        .filter(|n| n.kind() == "pair")
        .collect()
}

fn trailing_pair_prefix(raw: &[Node<'_>]) -> Option<usize> {
    if raw.len() >= 2 && raw[1..].iter().all(|n| n.kind() == "pair") {
        return Some(0);
    }
    let idx = raw.iter().position(|n| n.kind() == "pair")?;
    (idx > 0 && raw[idx..].iter().all(|n| n.kind() == "pair")).then_some(idx)
}

fn fixed_indent_argument_items<'a>(source: &SourceFile, raw: Vec<Node<'a>>) -> Vec<Node<'a>> {
    if raw.len() < 2 {
        return raw;
    }
    let mut items = raw[..raw.len() - 1].to_vec();
    let last = raw[raw.len() - 1];
    if is_bare_hash(source, last) {
        items.extend(hash_pairs(last));
    } else {
        items.push(last);
    }
    items
}

fn first_argument_items<'a>(source: &SourceFile, raw: Vec<Node<'a>>) -> Vec<Node<'a>> {
    let first = raw[0];
    if is_bare_hash(source, first) {
        return hash_pairs(first);
    }
    if let Some(idx) = trailing_pair_prefix(&raw) {
        return if idx == 0 {
            vec![first]
        } else {
            raw[..idx].to_vec()
        };
    }
    raw
}

/// RuboCop `flattened_arguments` for `Layout/ArgumentAlignment`.
fn collect_argument_items<'a>(source: &SourceFile, node: Node<'a>, style: &str) -> Vec<Node<'a>> {
    let raw = collect_items(node);
    if raw.is_empty() {
        return raw;
    }
    if style == "with_fixed_indentation" {
        fixed_indent_argument_items(source, raw)
    } else {
        first_argument_items(source, raw)
    }
}

/// Trailing keyword `pair`s in a call (implicit last-arg hash / kwargs).
fn trailing_keyword_pairs<'a>(node: Node<'a>) -> Vec<Node<'a>> {
    // Misparsed pattern-matching (`=>`) can yield errorful `argument_list`s of
    // pairs; RuboCop only inspects real hash nodes, so skip those.
    if node.has_error() {
        return Vec::new();
    }
    let raw = collect_items(node);
    let mut start = raw.len();
    while start > 0 && raw[start - 1].kind() == "pair" {
        start -= 1;
    }
    raw[start..].to_vec()
}

/// Items to align for `Layout/HashAlignment` (`hash` or kwargs in `argument_list`).
pub(crate) fn hash_alignment_items<'a>(node: Node<'a>) -> Vec<Node<'a>> {
    match node.kind() {
        "argument_list" | "command_argument_list" => trailing_keyword_pairs(node),
        "hash" => hash_pairs(node),
        _ => Vec::new(),
    }
}

pub(super) fn alignment_items<'a>(
    source: &SourceFile,
    node: Node<'a>,
    style: &str,
) -> Vec<Node<'a>> {
    if node.kind() == "argument_list" {
        collect_argument_items(source, node, style)
    } else {
        collect_items(node)
    }
}
