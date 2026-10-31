use super::context::*;
use crate::cop::CopRegistry;
use tree_sitter::Node;
mod escape_token;
mod options;
mod pattern;
mod scanner;
mod singleton;
use pattern::Pattern;
pub(super) fn register(r: &mut CopRegistry) {
    super::rules!(r;
        ("Style/RedundantRegexpCharacterClass", &["regex"], character_class, true, true, false),
        ("Style/RedundantRegexpEscape", &["regex"], escape, true, true, false)
    );
}
fn character_class(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let Some(pattern) = Pattern::new(c, n) else {
        return vec![];
    };
    pattern
        .sets()
        .into_iter()
        .filter_map(|(start, end, extended)| class_finding(&pattern, start, end, extended))
        .collect()
}
fn class_finding(p: &Pattern<'_>, start: usize, end: usize, extended: bool) -> Option<Finding> {
    let inner = &p.text[start + 1..end - 1];
    if !redundant_class(p, inner) {
        return None;
    }
    let replacement = if inner == "#" {
        "\\#".into()
    } else {
        inner.to_string()
    };
    let original = &p.text[start..end];
    let start = p.offset + start;
    let end = p.offset + end;
    let finding = Finding::range(
        start,
        end,
        format!(
            "Redundant single-element character class, `{original}` can be replaced with `{replacement}`."
        ),
    );
    Some(if unsafe_space(extended, inner) {
        finding
    } else {
        finding.edit(start, end, replacement)
    })
}
fn redundant_class(p: &Pattern<'_>, inner: &str) -> bool {
    singleton::singleton(inner)
        && !inner.starts_with('^')
        && !special_class(inner)
        && !(p.extended && inner.chars().any(char::is_whitespace))
}
fn special_class(inner: &str) -> bool {
    ["\\b", "\\1", "\\2", "\\3", "\\4", "\\5", "\\6", "\\7"].contains(&inner)
        || inner.chars().count() == 1 && ".*+?{}()|$".contains(inner)
}
fn escape(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let Some(pattern) = Pattern::new(c, n) else {
        return vec![];
    };
    pattern
        .escapes()
        .into_iter()
        .filter_map(|(index, inside)| escape_finding(&pattern, index, inside))
        .collect()
}
fn escape_finding(p: &Pattern<'_>, index: usize, inside: bool) -> Option<Finding> {
    let ch = p.text[index + 1..].chars().next()?;
    if allowed_escape(p, ch, index, inside) {
        return None;
    }
    let start = p.offset + index;
    Some(
        Finding::range(
            start,
            start + 1 + ch.len_utf8(),
            "Redundant escape inside regexp literal",
        )
        .edit(start, start + 1, ""),
    )
}
fn allowed_escape(p: &Pattern<'_>, ch: char, index: usize, inside: bool) -> bool {
    ch.is_alphanumeric()
        || " \n[]^\\#".contains(ch)
        || p.delimiters.contains(&ch)
        || !inside && ".*+?{}()|$".contains(ch)
        || inside && interior_hyphen(p, ch, index)
}
fn interior_hyphen(p: &Pattern<'_>, ch: char, index: usize) -> bool {
    ch == '-' && !opening_bracket(p, index) && p.text.as_bytes().get(index + 2) != Some(&b']')
}

fn opening_bracket(p: &Pattern<'_>, index: usize) -> bool {
    p.text.as_bytes().get(index.wrapping_sub(1)) == Some(&b'[')
        && (index < 2 || p.text.as_bytes().get(index - 2) != Some(&b'\\'))
}

fn unsafe_space(extended: bool, inner: &str) -> bool {
    extended && inner.chars().count() == 1 && inner.chars().any(char::is_whitespace)
}

pub(super) fn capture_counts(c: &Context<'_>, n: Node<'_>) -> Option<(usize, usize)> {
    if n.kind() != "regex" {
        return None;
    }
    Pattern::new(c, n)?.captures()
}
