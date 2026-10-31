use super::super::super::context::*;
use tree_sitter::Node;

pub(super) fn position(n: Node<'_>) -> Option<(usize, &'static str)> {
    let a = args(n)
        .into_iter()
        .filter(|n| n.kind() != "comment")
        .collect::<Vec<_>>();
    if uncertain_arguments(&a) {
        return None;
    }
    let last = *a.last()?;
    if last.kind() == "block_argument" {
        return None;
    }
    if last.kind() == "hash" {
        hash_position(last)
    } else {
        Some((last.end_byte(), ", "))
    }
}

fn uncertain_arguments(a: &[Node<'_>]) -> bool {
    a.is_empty()
        || a.iter().any(|n| uncertain(*n))
        || !a[1..].iter().all(|n| matches!(n.kind(), "pair" | "hash"))
}

fn uncertain(n: Node<'_>) -> bool {
    matches!(
        n.kind(),
        "splat_argument" | "hash_splat_argument" | "forward_argument" | "heredoc_beginning"
    ) || (n.kind() == "hash" && kids(n).iter().any(|n| uncertain(*n)))
        || (n.kind() == "pair" && !certain_key(n))
}

fn hash_position(n: Node<'_>) -> Option<(usize, &'static str)> {
    if let Some(last) = kids(n).last() {
        return Some((last.end_byte(), ", "));
    }
    let mut cursor = n.walk();
    n.children(&mut cursor)
        .find(|n| n.kind() == "}")
        .map(|n| (n.start_byte(), ""))
}

fn certain_key(n: Node<'_>) -> bool {
    field(n, "key").is_some_and(|key| matches!(key.kind(), "simple_symbol" | "hash_key_symbol"))
}
