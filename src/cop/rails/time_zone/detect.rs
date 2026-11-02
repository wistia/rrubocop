//! Time/Date call-chain helpers for Rails/TimeZone.

use std::sync::LazyLock;

use regex::Regex;
use tree_sitter::Node;

use crate::cop::shared::{call_method_name, node_bytes};
use crate::parse::source::SourceFile;

pub(super) const DANGEROUS: &[&[u8]] = &[b"now", b"local", b"new", b"parse", b"at"];
pub(super) const GOOD: &[&[u8]] = &[b"zone", b"zone_default", b"find_zone", b"find_zone!"];
pub(in crate::cop::rails) const ACCEPTED: &[&[u8]] = &[
    b"in_time_zone",
    b"utc",
    b"getlocal",
    b"xmlschema",
    b"iso8601",
    b"jisx0301",
    b"rfc3339",
    b"httpdate",
    b"to_i",
    b"to_f",
];

/// ISO `Z`, numeric UTC offsets, or common zone abbreviations — not any trailing letter
/// (avoids exempting `"March".to_time`).
static TZ_SPEC: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:\dZ|[+-]\d{2}:?\d{2}|\s(?:UTC|GMT|EST|EDT|CST|CDT|MST|MDT|PST|PDT))\z")
        .unwrap()
});

fn is_time_const(source: &SourceFile, node: Node<'_>) -> bool {
    match node.kind() {
        "constant" => node_bytes(source, node) == b"Time",
        "scope_resolution" => {
            node.child_by_field_name("name")
                .is_some_and(|n| n.kind() == "constant" && node_bytes(source, n) == b"Time")
                && node.child_by_field_name("scope").is_none()
        }
        _ => false,
    }
}

pub(super) fn time_receiver<'a>(source: &SourceFile, call: Node<'a>) -> Option<Node<'a>> {
    let recv = call.child_by_field_name("receiver")?;
    is_time_const(source, recv).then_some(recv)
}

pub(super) fn method_name<'a>(source: &'a SourceFile, call: Node<'a>) -> Option<&'a [u8]> {
    call_method_name(source, call)
}

fn pair_key_is_in(source: &SourceFile, pair: Node<'_>) -> bool {
    pair.child_by_field_name("key")
        .is_some_and(|key| matches!(node_bytes(source, key), b"in" | b"in:"))
}

pub(super) fn has_in_kwarg(source: &SourceFile, call: Node<'_>) -> bool {
    if let Some(args) = call.child_by_field_name("arguments") {
        let mut cur = args.walk();
        if args
            .named_children(&mut cur)
            .any(|c| c.kind() == "pair" && pair_key_is_in(source, c))
        {
            return true;
        }
    }
    let bytes = source.as_bytes();
    let (s, e) = (call.start_byte(), call.end_byte());
    e > s && e <= bytes.len() && bytes[s..e].windows(3).any(|w| w == b"in:")
}

pub(super) fn offset_provided(call: Node<'_>, method: &[u8]) -> bool {
    if method != b"new" && method != b"local" {
        return false;
    }
    let Some(args) = call.child_by_field_name("arguments") else {
        return false;
    };
    let mut cur = args.walk();
    args.named_children(&mut cur)
        .filter(|c| c.kind() != "comment")
        .count()
        >= 7
}

fn string_has_tz(source: &SourceFile, string_node: Node<'_>) -> bool {
    TZ_SPEC.is_match(
        String::from_utf8_lossy(node_bytes(source, string_node))
            .trim_matches(|c| c == '"' || c == '\''),
    )
}

pub(super) fn attach_tz_string(source: &SourceFile, call: Node<'_>) -> bool {
    let Some(args) = call.child_by_field_name("arguments") else {
        return false;
    };
    let mut cur = args.walk();
    args.named_children(&mut cur)
        .next()
        .is_some_and(|first| first.kind() == "string" && string_has_tz(source, first))
}

fn has_safe_nav(source: &SourceFile, recv: Node<'_>, parent: Node<'_>) -> bool {
    source
        .as_bytes()
        .get(
            recv.end_byte()
                ..parent
                    .child_by_field_name("method")
                    .map(|m| m.start_byte())
                    .unwrap_or(parent.end_byte()),
        )
        .is_some_and(|b| b.windows(2).any(|w| w == b"&."))
}

/// Walk parent call chain while receiver traces back to this Time call.
pub(super) fn chain_from(source: &SourceFile, call: Node<'_>) -> Vec<Vec<u8>> {
    let mut chain = Vec::new();
    if let Some(m) = method_name(source, call) {
        chain.push(m.to_vec());
    }
    let mut cur = call;
    while let Some(parent) = cur.parent() {
        if parent.kind() != "call" {
            break;
        }
        let Some(recv) = parent.child_by_field_name("receiver") else {
            break;
        };
        if recv.id() != cur.id() || has_safe_nav(source, recv, parent) {
            break;
        }
        if let Some(m) = method_name(source, parent) {
            chain.push(m.to_vec());
        }
        cur = parent;
    }
    chain
}

pub(super) fn not_danger_chain(chain: &[Vec<u8>], flexible: bool) -> bool {
    chain.iter().any(|m| {
        let mb = m.as_slice();
        GOOD.iter().any(|&g| g == mb)
            || (flexible && (ACCEPTED.iter().any(|&g| g == mb) || mb == b"current"))
    })
}

fn acceptable_list(method: &str) -> String {
    let mut parts = vec![format!("`Time.zone.{method}`")];
    if method != "current" {
        parts.push("`Time.current`".into());
    }
    for a in ACCEPTED {
        parts.push(format!("`Time.{method}.{}`", String::from_utf8_lossy(a)));
    }
    parts.join(", ")
}

pub(super) fn build_message(flexible: bool, method: &str) -> String {
    if flexible {
        format!(
            "Do not use `Time.{method}` without zone. Use one of {} instead.",
            acceptable_list(method)
        )
    } else {
        let prefer = if method == "current" { "now" } else { method };
        format!("Do not use `Time.{method}` without zone. Use `Time.zone.{prefer}` instead.")
    }
}

pub(super) fn selector_off(call: Node<'_>) -> usize {
    call.child_by_field_name("method")
        .map(|m| m.start_byte())
        .unwrap_or_else(|| call.start_byte())
}

pub(super) fn string_to_time_needs_zone(source: &SourceFile, node: Node<'_>) -> bool {
    method_name(source, node) == Some(b"to_time")
        && node
            .child_by_field_name("receiver")
            .is_some_and(|recv| recv.kind() == "string" && !string_has_tz(source, recv))
}
