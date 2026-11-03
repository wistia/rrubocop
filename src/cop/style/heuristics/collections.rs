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

/// RuboCop `/\A[_a-z]\w*[?!]?\z/i` on a symbol name (no leading `:`).
pub(crate) fn word_symbol_name(name: &[u8]) -> bool {
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
    body.bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

#[cfg(test)]
mod hash_symbol_tests {
    use super::word_symbol_name;

    #[test]
    fn original_ruby_case_insensitive_word_pattern() {
        for (symbol, expected) in [
            ("aé", false),
            ("é", false),
            ("K", true),
            ("aK", false),
            ("ſ", true),
            ("aſ", false),
            ("a_b", true),
            ("a!", true),
            ("a?", true),
            ("a?!", false),
            ("", false),
            ("_", true),
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
