mod correction;
use super::{ACCEPTED_METHODS, TimeZone};
use crate::cop::shared::ruby_arguments::{Argument, from_nodes};
use crate::cop::shared::ruby_expression::{
    plain_string, plain_symbol, regular_send_arguments, regular_send_name, regular_send_receiver,
    send_name,
};
use crate::cop::shared::{call_method_name, method_node, node_bytes, push_replace};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

const DANGEROUS: &[&[u8]] = &[b"now", b"local", b"new", b"parse", b"at"];
const GOOD: &[&[u8]] = &[b"zone", b"zone_default", b"find_zone", b"find_zone!"];
fn target(node: Node<'_>) -> Node<'_> {
    if node.kind() == "assignment" {
        node.child_by_field_name("left").unwrap_or(node)
    } else {
        node
    }
}
fn arguments<'n>(source: &SourceFile, node: Node<'n>) -> Vec<Argument<'n>> {
    from_nodes(regular_send_arguments(source, node).unwrap_or_default())
}
fn time_constant(source: &SourceFile, node: Node<'_>) -> bool {
    if node.kind() == "constant" {
        node_bytes(source, node) == b"Time"
            && !node.parent().is_some_and(|p| {
                p.kind() == "scope_resolution" && p.child_by_field_name("name") == Some(node)
            })
    } else {
        node.kind() == "scope_resolution"
            && node.child_by_field_name("scope").is_none()
            && node
                .child_by_field_name("name")
                .is_some_and(|name| node_bytes(source, name) == b"Time")
    }
}
fn send_parent<'n>(source: &SourceFile, node: Node<'n>) -> Option<Node<'n>> {
    if target(node).child_by_field_name("block").is_some() {
        return None;
    }
    let mut parent = node.parent()?;
    if parent.kind() == "argument_list" {
        parent = parent.parent()?;
    }
    regular_send_name(source, parent)?;
    Some(parent)
}
fn from_time(source: &SourceFile, node: Node<'_>) -> bool {
    if time_constant(source, node) {
        return true;
    }
    regular_send_receiver(source, node)
        .flatten()
        .is_some_and(|receiver| from_time(source, receiver))
}
fn chain<'n>(source: &SourceFile, node: Node<'n>) -> Vec<Node<'n>> {
    let mut result = vec![node];
    let mut next = send_parent(source, node);
    while let Some(parent) = next {
        result.push(parent);
        next = send_parent(source, parent);
    }
    result
}
fn selector(node: Node<'_>) -> Option<(usize, usize)> {
    let node = target(node);
    if let Some(token) = method_node(node).or_else(|| node.child_by_field_name("operator")) {
        return Some((token.start_byte(), token.end_byte()));
    }
    if node.kind() == "element_reference" {
        let mut cursor = node.walk();
        let brackets = node
            .children(&mut cursor)
            .filter(|child| matches!(child.kind(), "[" | "]"))
            .collect::<Vec<_>>();
        return Some((brackets.first()?.start_byte(), brackets.last()?.end_byte()));
    }
    None
}
fn file_literal(source: &SourceFile, node: Node<'_>) -> bool {
    node.kind() == "identifier" && node_bytes(source, node) == b"__FILE__"
}
pub(super) fn attach_timezone(source: &SourceFile, node: Node<'_>) -> bool {
    let value = if file_literal(source, node) {
        Some(source.path_str().to_owned())
    } else {
        super::super::literal_value::value(source, node)
    };
    let Some(value) = value else {
        return false;
    };
    let bytes = value.as_bytes();
    if bytes.last().is_some_and(u8::is_ascii_alphabetic) {
        return true;
    }
    [5, 6].into_iter().any(|length| {
        bytes.len() >= length && {
            let tail = &bytes[bytes.len() - length..];
            matches!(tail[0], b'+' | b'-')
                && tail[1..3].iter().all(u8::is_ascii_digit)
                && if length == 6 {
                    tail[3] == b':' && tail[4..].iter().all(u8::is_ascii_digit)
                } else {
                    tail[3..].iter().all(u8::is_ascii_digit)
                }
        }
    })
}
fn offset_provided(source: &SourceFile, node: Node<'_>) -> bool {
    let Some(method) = regular_send_name(source, node) else {
        return false;
    };
    let args = arguments(source, node);
    if method.as_ref() == b"new" && args.len() == 7 {
        return true;
    }
    if !matches!(method.as_ref(), b"new" | b"at" | b"now") {
        return false;
    }
    args.last().filter(|arg| arg.hash).is_some_and(|hash| {
        hash.parts().iter().any(|pair| {
            pair.kind() == "pair"
                && pair
                    .child_by_field_name("key")
                    .and_then(|key| plain_symbol(source, key))
                    .as_deref()
                    == Some("in")
                && pair
                    .child_by_field_name("value")
                    .is_some_and(|value| value.kind() != "nil")
        })
    })
}
fn safe_method<'a>(method: &'a str, has_arguments: bool) -> &'a str {
    if matches!(method, "new" | "current") {
        if has_arguments { "local" } else { "now" }
    } else {
        method
    }
}
fn message(method: &str, has_arguments: bool, flexible: bool) -> String {
    let preferred = safe_method(method, has_arguments);
    if !flexible {
        return format!(
            "Do not use `Time.{method}` without zone. Use `Time.zone.{preferred}` instead."
        );
    }
    let mut choices = vec![format!("`Time.zone.{preferred}`"), "`Time.current`".into()];
    choices.extend(
        ACCEPTED_METHODS
            .iter()
            .map(|accepted| format!("`Time.{method}.{}`", String::from_utf8_lossy(accepted))),
    );
    format!(
        "Do not use `Time.{method}` without zone. Use one of {} instead.",
        choices.join(", ")
    )
}
fn report(
    cop: &TimeZone,
    source: &SourceFile,
    node: Node<'_>,
    text: String,
    diagnostics: &mut Vec<Diagnostic>,
    edits: Option<correction::Edits>,
    config: &CopConfig,
    mut corrections: Option<&mut Vec<Correction>>,
) {
    let Some((start, end)) = selector(node) else {
        return;
    };
    let (line, col) = source.offset_to_line_col(start);
    let mut d = cop.diagnostic(source, line, col, text);
    d.highlight_length = super::super::blank::highlight_length(source, start, end);
    d.correctable = edits.is_some() && !config.autocorrect_disabled();
    if d.correctable {
        for (start, end, replacement) in edits.unwrap() {
            d.corrected |= push_replace(&mut corrections, start, end, replacement, cop.name());
        }
    }
    diagnostics.push(d);
}
pub(super) fn check(
    cop: &TimeZone,
    source: &SourceFile,
    node: Node<'_>,
    config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: Option<&mut Vec<Correction>>,
) {
    if node.kind() == "call"
        && call_method_name(source, node) == Some(b"to_time")
        && send_name(source, node).as_deref() == Some(b"to_time")
        && node
            .child_by_field_name("receiver")
            .is_some_and(|receiver| {
                plain_string(source, receiver).is_some() || file_literal(source, receiver)
            })
    {
        report(
            cop,
            source,
            node,
            "Do not use `String#to_time` without zone. Use `Time.zone.parse` instead.".into(),
            diagnostics,
            correction::string(source, node),
            config,
            corrections,
        );
        return;
    }
    if !time_constant(source, node) {
        return;
    }
    let Some(mut call) = node.parent() else {
        return;
    };
    if let Some(parent) = call
        .parent()
        .filter(|p| p.kind() == "assignment" && p.child_by_field_name("left") == Some(call))
    {
        call = parent;
    }
    if regular_send_receiver(source, call) != Some(Some(node)) {
        return;
    }
    let args = arguments(source, call);
    if args
        .first()
        .and_then(Argument::node)
        .is_some_and(|arg| attach_timezone(source, arg))
    {
        return;
    }
    let flexible = config.get_str("EnforcedStyle", "flexible") == "flexible";
    let nodes = chain(source, call);
    let names = nodes
        .iter()
        .filter(|node| from_time(source, **node))
        .filter_map(|node| regular_send_name(source, *node))
        .collect::<Vec<_>>();
    if !names.iter().any(|name| DANGEROUS.contains(&name.as_ref()))
        || names.iter().any(|name| {
            GOOD.contains(&name.as_ref())
                || (flexible
                    && (name.as_ref() == b"current" || ACCEPTED_METHODS.contains(&name.as_ref())))
        })
    {
        return;
    }
    let text = if flexible && names.iter().any(|name| name.as_ref() == b"localtime") {
        let Some(localtime) = nodes
            .iter()
            .find(|node| regular_send_name(source, **node).as_deref() == Some(b"localtime"))
        else {
            return;
        };
        if !arguments(source, *localtime).is_empty() {
            return;
        }
        "Do not use `Time.localtime` without offset or zone.".into()
    } else {
        if offset_provided(source, call) {
            return;
        }
        let mut dangerous = Vec::<String>::new();
        for name in names
            .iter()
            .filter(|name| DANGEROUS.contains(&name.as_ref()))
        {
            let name = String::from_utf8_lossy(name).into_owned();
            if !dangerous.contains(&name) {
                dangerous.push(name);
            }
        }
        message(&dangerous.join("."), !args.is_empty(), flexible)
    };
    report(
        cop,
        source,
        call,
        text,
        diagnostics,
        correction::time(source, call, !flexible),
        config,
        corrections,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn timezone_suffix_predicate_matches_original_scalar_values() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/framework-time-zone-values.json"
        )))
        .unwrap();
        for row in fixture["cases"].as_array().unwrap() {
            let text = format!("Time.new({})", row["source"].as_str().unwrap());
            let source = SourceFile::from_bytes("app/models/example.rb", text.into_bytes());
            let tree = crate::parse::parse_ruby(&source).unwrap();
            let call = tree.root_node().named_child(0).unwrap();
            let argument = arguments(&source, call)[0].node().unwrap();
            assert_eq!(
                attach_timezone(&source, argument),
                row["attach"].as_bool().unwrap(),
                "{}",
                row["source"]
            );
        }
    }
}
