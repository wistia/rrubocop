use super::super::context::*;
use tree_sitter::Node;
pub(super) fn interpolation(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let text = c.text(n);
    if interpolated(n) || !has_interpolation(text) {
        return vec![];
    }
    let Some(content) = quoted_content(text) else {
        return vec![];
    };
    let replacement = || {
        if text.contains('"') {
            format!("%{{{content}}}")
        } else {
            format!("\"{content}\"")
        }
    };
    vec![Finding::new(n, "Interpolation in single quoted string detected. Use double quoted strings if you need interpolation.").replace_with(c, n, replacement)]
}
fn interpolated(n: Node<'_>) -> bool {
    descendants(n).any(|d| d.kind() == "interpolation")
}
fn quoted_content(text: &str) -> Option<&str> {
    if text.starts_with('\'') {
        Some(&text[1..text.len() - 1])
    } else if text.starts_with("%q") {
        Some(&text[3..text.len() - 1])
    } else {
        None
    }
}
fn has_interpolation(s: &str) -> bool {
    s.match_indices("#{")
        .any(|(i, _)| (i == 0 || s.as_bytes()[i - 1] != b'\\') && s[i + 2..].contains('}'))
}
