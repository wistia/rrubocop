use super::super::context::*;
use std::collections::HashSet;
use std::sync::LazyLock;
static DEPARTMENTS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    crate::cop::CopRegistry::default_registry()
        .names()
        .into_iter()
        .filter_map(|n| n.split_once('/').map(|(dept, _)| dept))
        .collect()
});
use regex::Regex;
use tree_sitter::Node;
static DIRECTIVE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(# *rubocop *: *(?:disable|enable|todo) +)(.*)").unwrap());
static TOKENS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^,]+|[^A-Za-z0-9_]+").unwrap());
static QUALIFIED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[A-Za-z]+/[A-Za-z]+|all").unwrap());
static NONWORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^A-Za-z0-9_]+").unwrap());
pub(super) fn check(c: &Context<'_>, n: Node<'_>) -> Vec<Finding> {
    let Some(captures) = DIRECTIVE.captures(c.text(n)) else {
        return vec![];
    };
    let rest = captures.get(2).unwrap();
    let mut out = vec![];
    for token in TOKENS.find_iter(rest.as_str()) {
        if let Some(finding) = check_token(c, n, rest.start(), &token) {
            out.push(finding);
        }
        if unexpected(token.as_str()) {
            break;
        }
    }
    out
}
fn valid(name: &str) -> bool {
    NONWORD.is_match(name) || QUALIFIED.is_match(name) || DEPARTMENTS.contains(name)
}
fn qualify(name: &str) -> Option<String> {
    crate::config::cop_names::qualified(name)
        .map(|s| s.into_owned())
        .or_else(|| {
            include_str!("../../../resources/legacy_cop_names.txt")
                .lines()
                .find(|n| n.split_once('/').is_some_and(|(_, short)| short == name))
                .map(str::to_owned)
        })
}

fn check_token(
    c: &Context<'_>,
    n: Node<'_>,
    offset: usize,
    token: &regex::Match<'_>,
) -> Option<Finding> {
    let name = token
        .as_str()
        .trim_matches(|ch| matches!(ch, '\0' | '\t' | '\n' | '\x0b' | '\x0c' | '\r' | ' '));
    if valid(name) {
        return None;
    }
    let start = n.start_byte() + offset + token.start();
    let f = Finding::range(start, start + name.len(), "Department name is missing.");
    Some(
        if let Some(qualified) =
            qualify(name).filter(|_| token.as_str().get(..name.len()) == Some(name))
        {
            f.edit_with(c, start, start + name.len(), || qualified)
        } else {
            f
        },
    )
}
fn unexpected(name: &str) -> bool {
    name.bytes()
        .any(|b| !b.is_ascii_alphabetic() && !matches!(b, b'/' | b',' | b' '))
}
