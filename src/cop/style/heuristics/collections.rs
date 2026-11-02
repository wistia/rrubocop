//! Heuristic matchers for breadth-first Style cops.
use crate::cop::CopConfig;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub fn matches_empty_heredoc(source: &SourceFile, node: Node<'_>, _config: &CopConfig) -> bool {
    if node.kind() != "heredoc_body" {
        return false;
    }
    let mut cur = node.walk();
    node.children(&mut cur)
        .find(|ch| ch.kind() == "heredoc_content")
        .is_some_and(|c| {
            let b = &source.as_bytes()[c.start_byte()..c.end_byte()];
            b.is_empty() || b == b"\n"
        })
}

pub fn matches_hash_syntax(source: &SourceFile, node: Node<'_>, config: &CopConfig) -> bool {
    if node.kind() != "pair" {
        return false;
    }
    let style = config.get_str("EnforcedStyle", "ruby19");
    let has_rocket = has_anon_child(node, "=>");
    match style {
        // RuboCop ruby19: only rewrite when *every* key is a word symbol.
        "ruby19" | "ruby19_no_mixed_keys" => {
            has_rocket
                && pair_key_is_symbol(source, node)
                && hash_all_word_symbol_keys(source, node)
        }
        "hash_rockets" => has_anon_child(node, ":"),
        _ => false,
    }
}

fn hash_all_word_symbol_keys(source: &SourceFile, pair: Node<'_>) -> bool {
    let pairs = sibling_pairs(pair);
    !pairs.is_empty() && pairs.iter().all(|p| pair_key_is_symbol(source, *p))
}

fn sibling_pairs(pair: Node<'_>) -> Vec<Node<'_>> {
    let Some(parent) = pair.parent() else {
        return vec![pair];
    };
    if !matches!(parent.kind(), "hash" | "argument_list") {
        return vec![pair];
    }
    let mut cur = parent.walk();
    parent
        .named_children(&mut cur)
        .filter(|n| n.kind() == "pair")
        .collect()
}

fn has_anon_child(node: Node<'_>, kind: &str) -> bool {
    let mut cur = node.walk();
    node.children(&mut cur)
        .any(|ch| !ch.is_named() && ch.kind() == kind)
}

fn pair_key_is_symbol(source: &SourceFile, node: Node<'_>) -> bool {
    let mut c3 = node.walk();
    let Some(key) = node.named_children(&mut c3).next() else {
        return false;
    };
    if !matches!(key.kind(), "simple_symbol" | "hash_key_symbol") {
        return false;
    }
    let bytes = &source.as_bytes()[key.start_byte()..key.end_byte()];
    // RuboCop `acceptable_19_syntax_symbol?`: word symbols only (not :@ivar / :$g / :foo=).
    acceptable_19_symbol(bytes)
}

fn acceptable_19_symbol(bytes: &[u8]) -> bool {
    let name = strip_leading_colon(bytes);
    // Setter symbols and non-word forms cannot use `key:` label syntax.
    if name.ends_with(b"=") || name.starts_with(b"@") || name.starts_with(b"$") {
        return false;
    }
    if name.starts_with(b"'") || name.starts_with(b"\"") {
        return true;
    }
    word_symbol_name(name)
}

fn strip_leading_colon(bytes: &[u8]) -> &[u8] {
    if bytes.starts_with(b":") {
        &bytes[1..]
    } else {
        bytes
    }
}

/// RuboCop `/\A[_a-z]\w*[?!]?\z/i` on a symbol name (no leading `:`).
fn word_symbol_name(name: &[u8]) -> bool {
    let Ok(s) = std::str::from_utf8(name) else {
        return false;
    };
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if first != '_' && !first.is_ascii_alphabetic() && !matches!(first, 'K' | 'ſ') {
        return false;
    }
    let rest = chars.as_str();
    let body = rest
        .strip_suffix('?')
        .or_else(|| rest.strip_suffix('!'))
        .unwrap_or(rest);
    body.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

#[cfg(test)]
mod hash_symbol_tests {
    use super::word_symbol_name;

    #[test]
    fn original_ruby_case_insensitive_word_pattern() {
        for (symbol, expected) in [
            ("aé", false), ("é", false), ("K", true), ("aK", false),
            ("ſ", true), ("aſ", false), ("a_b", true), ("a!", true),
            ("a?", true), ("a?!", false), ("", false), ("_", true),
        ] {
            assert_eq!(word_symbol_name(symbol.as_bytes()), expected, "{symbol}");
        }
    }
}

pub fn matches_symbol_array(_source: &SourceFile, node: Node<'_>, config: &CopConfig) -> bool {
    let style = config.get_str("EnforcedStyle", "percent_i");
    match node.kind() {
        "array" => {
            if style != "percent_i" {
                return false;
            }
            let mut cur = node.walk();
            let elems: Vec<_> = node.named_children(&mut cur).collect();
            if elems.len() < 2 {
                return false;
            }
            elems
                .iter()
                .all(|e| matches!(e.kind(), "simple_symbol" | "delimited_symbol"))
        }
        "symbol_array" => style == "brackets",
        _ => false,
    }
}

pub fn matches_word_array(_source: &SourceFile, node: Node<'_>, config: &CopConfig) -> bool {
    let style = config.get_str("EnforcedStyle", "percent_w");
    match node.kind() {
        "array" => {
            if style != "percent_w" {
                return false;
            }
            let mut cur = node.walk();
            let elems: Vec<_> = node.named_children(&mut cur).collect();
            if elems.len() < 2 {
                return false;
            }
            elems.iter().all(|e| e.kind() == "string")
        }
        "string_array" => style == "brackets",
        _ => false,
    }
}
