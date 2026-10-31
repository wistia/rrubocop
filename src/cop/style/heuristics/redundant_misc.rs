//! Heuristic matchers for breadth-first Style cops.
use crate::cop::CopConfig;
use crate::cop::style::heuristics::double_quotes_required;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub fn matches_redundant_capital_w(
    source: &SourceFile,
    node: Node<'_>,
    _config: &CopConfig,
) -> bool {
    if node.kind() != "string_array" {
        return false;
    }
    let b = &source.as_bytes()[node.start_byte()..node.end_byte()];
    b.starts_with(b"%W")
}

pub fn matches_redundant_double_splat_hash_braces(
    _source: &SourceFile,
    node: Node<'_>,
    _config: &CopConfig,
) -> bool {
    if node.kind() != "hash_splat_argument" {
        return false;
    }
    let mut cur = node.walk();
    node.named_children(&mut cur).any(|ch| {
        ch.kind() == "hash" && {
            let mut c2 = ch.walk();
            ch.named_children(&mut c2).next().is_none()
        }
    })
}

pub fn matches_redundant_heredoc_delimiter_quotes(
    source: &SourceFile,
    node: Node<'_>,
    _config: &CopConfig,
) -> bool {
    if node.kind() != "heredoc_beginning" {
        return false;
    }
    let b = &source.as_bytes()[node.start_byte()..node.end_byte()];
    if !b.contains(&b'\'') && !b.contains(&b'"') {
        return false;
    }
    if !b.contains(&b'\'') {
        return true;
    }
    heredoc_quotes_needed(source, node)
}

pub fn matches_redundant_interpolation(
    _source: &SourceFile,
    node: Node<'_>,
    _config: &CopConfig,
) -> bool {
    let Some(parent) = sole_interp_string(node) else {
        return false;
    };
    !skip_redundant_interp_string(parent)
}

/// Cop visits `interpolation`; parent must be a string with only that interp.
fn sole_interp_string(node: Node<'_>) -> Option<Node<'_>> {
    if node.kind() != "interpolation" {
        return None;
    }
    let parent = node.parent()?;
    if parent.kind() != "string" {
        return None;
    }
    let mut cur = parent.walk();
    let kids: Vec<_> = parent.named_children(&mut cur).collect();
    (kids.len() == 1 && kids[0].kind() == "interpolation").then_some(parent)
}

fn skip_redundant_interp_string(string: Node<'_>) -> bool {
    // RuboCop skips parts of implicit concatenation (`"a" "b"` / line-continued).
    if string
        .parent()
        .is_some_and(|p| p.kind() == "chained_string")
    {
        return true;
    }
    // `"#{x}": value` hash labels are not `dstr` in RuboCop's parser — don't flag.
    string.parent().is_some_and(|p| {
        p.kind() == "pair"
            && p.child_by_field_name("key")
                .is_some_and(|k| k.id() == string.id())
    })
}

/// RuboCop Style/RedundantPercentQ: `%q`/`%Q` only when `'`/`"` would work equally.
pub fn matches_redundant_percent_q(
    source: &SourceFile,
    node: Node<'_>,
    _config: &CopConfig,
) -> bool {
    if node.kind() != "string" {
        return false;
    }
    let b = &source.as_bytes()[node.start_byte()..node.end_byte()];
    let Some(kind) = percent_q_kind(b) else {
        return false;
    };
    // Both `'` and `"` appear in the literal → keep percent form.
    if b.contains(&b'\'') && b.contains(&b'"') {
        return false;
    }
    match kind {
        PercentQKind::Lower => !acceptable_q(b),
        PercentQKind::Upper => !acceptable_capital_q(node, b),
    }
}

enum PercentQKind {
    Lower,
    Upper,
}

fn percent_q_kind(b: &[u8]) -> Option<PercentQKind> {
    if b.starts_with(b"%Q") {
        Some(PercentQKind::Upper)
    } else if b.starts_with(b"%q") {
        Some(PercentQKind::Lower)
    } else {
        None
    }
}

fn has_interpolation_text(src: &[u8]) -> bool {
    // RuboCop STRING_INTERPOLATION_REGEXP = /#\{.+\}/
    let s = std::str::from_utf8(src).unwrap_or("");
    s.contains("#{") && s.contains('}')
}

fn escaped_non_backslash(src: &[u8]) -> bool {
    let mut i = 0;
    while i + 1 < src.len() {
        if src[i] == b'\\' {
            if src[i + 1] != b'\\' {
                return true;
            }
            i += 2;
        } else {
            i += 1;
        }
    }
    false
}

fn acceptable_q(src: &[u8]) -> bool {
    has_interpolation_text(src) || escaped_non_backslash(src)
}

fn acceptable_capital_q(node: Node<'_>, src: &[u8]) -> bool {
    if !src.contains(&b'"') {
        return false;
    }
    if has_interpolation_text(src) {
        return true;
    }
    // Static %Q — allowed when double quotes are required (e.g. `\n` escapes).
    let mut cur = node.walk();
    !node
        .named_children(&mut cur)
        .any(|ch| ch.kind() == "interpolation")
        && double_quotes_required(src)
}

fn heredoc_quotes_needed(source: &SourceFile, node: Node<'_>) -> bool {
    let mut sibling = node.next_named_sibling();
    while let Some(n) = sibling {
        if n.kind() == "heredoc_body" {
            let text = &source.as_bytes()[n.start_byte()..n.end_byte()];
            return !text.contains(&b'\\') && !text.windows(2).any(|w| w == b"#{");
        }
        sibling = n.next_named_sibling();
    }
    false
}
