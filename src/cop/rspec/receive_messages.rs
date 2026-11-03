use crate::cop::rspec::helpers::{RSPEC_INCLUDE, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::rspec::ruby_arguments::{Argument, arguments};
use crate::cop::shared::{call_receiver, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

struct Stub<'a> {
    node: Node<'a>,
    allow: Node<'a>,
    allow_arguments: Vec<Argument<'a>>,
    matcher: Node<'a>,
    message: String,
    value: Argument<'a>,
}
fn stub<'a>(source: &SourceFile, node: Node<'a>) -> Option<Stub<'a>> {
    if name(source, node) != Some(b"to") || safe_send(source, node) || call_block(node).is_some() {
        return None;
    }
    let allow =
        call_receiver(node).filter(|n| bare(source, *n, b"allow") && call_block(*n).is_none())?;
    let args = arguments(node);
    if args.len() != 1 {
        return None;
    }
    let matcher = args[0].node()?;
    if name(source, matcher) != Some(b"and_return")
        || safe_send(source, matcher)
        || call_block(matcher).is_some()
    {
        return None;
    }
    let receive = call_receiver(matcher)
        .filter(|n| bare(source, *n, b"receive") && call_block(*n).is_none())?;
    let messages = call_args(receive);
    if messages.len() != 1 {
        return None;
    }
    let message = crate::cop::shared::ruby_expression::plain_symbol(source, messages[0])?;
    let mut values = arguments(matcher);
    if values.len() != 1 {
        return None;
    }
    let value = values.remove(0);
    if value.node().is_some_and(|n| {
        n.kind() == "splat_argument"
            || n.kind() == "heredoc_beginning" && !node_bytes(source, n).contains(&b'`')
    }) {
        return None;
    }
    Some(Stub {
        node,
        allow,
        allow_arguments: arguments(allow),
        matcher,
        message,
        value,
    })
}
fn equal_first(source: &SourceFile, left: &[Argument<'_>], right: &[Argument<'_>]) -> bool {
    match (left.first(), right.first()) {
        (None, None) => true,
        (Some(a), Some(b)) if a.hash && b.hash => {
            let a = a.parts();
            let b = b.parts();
            a.len() == b.len()
                && a.iter()
                    .zip(&b)
                    .all(|(a, b)| eql_expression(source, *a, *b))
        }
        (Some(a), Some(b)) if !a.hash && !b.hash => a
            .node()
            .zip(b.node())
            .is_some_and(|(a, b)| eql_expression(source, a, b)),
        _ => false,
    }
}
fn line_range(source: &SourceFile, node: Node<'_>) -> Option<(usize, usize)> {
    let bytes = source.as_bytes();
    let start = bytes[..node.start_byte()]
        .iter()
        .rposition(|b| *b == b'\n')
        .map_or(0, |p| p + 1);
    let end = bytes[node.end_byte()..]
        .iter()
        .position(|b| *b == b'\n')
        .map_or(bytes.len(), |p| node.end_byte() + p + 1);
    (bytes[start..node.start_byte()]
        .iter()
        .all(|b| b.is_ascii_whitespace())
        && bytes[node.end_byte()..end]
            .iter()
            .all(|b| b.is_ascii_whitespace()))
    .then_some((start, end))
}
fn label(message: &str) -> Option<String> {
    let nonword = !message.is_empty()
        && message
            .chars()
            .all(|c| !(c.is_ascii_alphanumeric() || c == '_'));
    let label = if message.ends_with('=') || nonword || message.starts_with(":\"") {
        format!("'{message}'")
    } else {
        message.into()
    };
    let source = SourceFile::from_bytes("label.rb", format!("{{{label}: 0}}").into_bytes());
    let tree = crate::parse::parse_ruby(&source).ok()?;
    if tree.root_node().has_error() {
        return None;
    }
    let key = tree
        .root_node()
        .named_child(0)?
        .named_child(0)?
        .child_by_field_name("key")?;
    (crate::cop::ported::static_symbol(&source, key).as_deref() == Some(message)).then_some(label)
}
fn safe_value(source: &SourceFile, value: &Argument<'_>) -> bool {
    value.nodes.iter().all(|node| {
        pure_literal(source, *node)
            || node.kind() == "identifier" && crate::cop::ported::local_identifier(source, *node)
    })
}
fn constant_path(node: Node<'_>) -> bool {
    node.kind() == "constant"
        || node.kind() == "scope_resolution"
            && node
                .child_by_field_name("name")
                .is_some_and(|name| name.kind() == "constant")
            && node.child_by_field_name("scope").is_none_or(constant_path)
}
fn stable_allow(source: &SourceFile, allow: Node<'_>) -> bool {
    let args = arguments(allow);
    args.len() == 1
        && !args[0].hash
        && args[0].node().is_some_and(|node| {
            node.kind() == "self"
                || constant_path(node)
                || node.kind() == "identifier" && crate::cop::ported::local_identifier(source, node)
        })
}
fn replacement(
    source: &SourceFile,
    siblings: &[Node<'_>],
    items: &[&Stub<'_>],
) -> Option<(usize, usize, String)> {
    let first = items.first()?;
    let last = items.last()?;
    if !stable_allow(source, first.allow)
        || items.iter().any(|item| !safe_value(source, &item.value))
    {
        return None;
    }
    let start_index = siblings.iter().position(|n| *n == first.node)?;
    let end_index = siblings.iter().position(|n| *n == last.node)?;
    if end_index - start_index + 1 != items.len() {
        return None;
    }
    let mut ranges = items
        .iter()
        .map(|item| line_range(source, item.node))
        .collect::<Option<Vec<_>>>()?;
    if ranges.windows(2).any(|pair| pair[0].1 > pair[1].0) {
        return None;
    }
    let start = ranges[0].0;
    let end = ranges.last()?.1;
    if crate::cop::shared::collect_comments(siblings[start_index].parent()?)
        .iter()
        .any(|comment| comment.start_byte() >= start && comment.start_byte() < end)
    {
        return None;
    }
    let pairs = items
        .iter()
        .map(|item| {
            let label = label(&item.message)?;
            let raw =
                String::from_utf8_lossy(&source.as_bytes()[item.value.start()..item.value.end()]);
            let value = if item.value.hash && !item.value.braces {
                format!("{{ {raw} }}")
            } else {
                raw.into_owned()
            };
            Some(format!("{label}: {value}"))
        })
        .collect::<Option<Vec<_>>>()?;
    let mut text = source.as_bytes()[start..end].to_vec();
    text.splice(
        last.matcher.start_byte() - start..last.matcher.end_byte() - start,
        format!("receive_messages({})", pairs.join(", ")).bytes(),
    );
    ranges.pop();
    for (a, b) in ranges.into_iter().rev() {
        text.drain(a - start..b - start);
    }
    let text = String::from_utf8(text).ok()?;
    let parsed = SourceFile::from_bytes("stubs.rb", text.as_bytes().to_vec());
    crate::parse::parse_ruby(&parsed)
        .ok()
        .filter(|tree| !tree.root_node().has_error())?;
    Some((start, end, text))
}

pub struct ReceiveMessages;
impl Cop for ReceiveMessages {
    fn name(&self) -> &'static str {
        "RSpec/ReceiveMessages"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &[
            "program",
            "body_statement",
            "block_body",
            "then",
            "else",
            "begin",
            "parenthesized_statements",
        ]
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn safe_autocorrect(&self) -> bool {
        false
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let siblings = code_children(node);
        if node.kind() == "begin"
            && !siblings
                .iter()
                .any(|child| matches!(child.kind(), "rescue" | "else" | "ensure"))
        {
            return;
        }
        if siblings.len() < 2 {
            return;
        }
        let candidates = siblings
            .iter()
            .filter_map(|n| stub(source, *n))
            .collect::<Vec<_>>();
        let mut groups: Vec<Vec<&Stub<'_>>> = Vec::new();
        for candidate in &candidates {
            if let Some(group) = groups.iter_mut().find(|group| {
                equal_first(
                    source,
                    &group[0].allow_arguments,
                    &candidate.allow_arguments,
                )
            }) {
                group.push(candidate);
            } else {
                groups.push(vec![candidate]);
            }
        }
        for group in groups {
            let mut message_lines: std::collections::HashMap<&str, (usize, bool)> =
                std::collections::HashMap::new();
            for item in &group {
                let line = item.node.start_position().row;
                message_lines
                    .entry(&item.message)
                    .and_modify(|(first, multiple)| *multiple |= *first != line)
                    .or_insert((line, false));
            }
            let items = group
                .iter()
                .copied()
                .filter(|item| !message_lines[item.message.as_str()].1)
                .collect::<Vec<_>>();
            let mut owners = Vec::new();
            for item in &items {
                let lines = items
                    .iter()
                    .filter(|other| {
                        other.node.start_position().row != item.node.start_position().row
                    })
                    .map(|other| other.node.start_position().row + 1)
                    .collect::<Vec<_>>();
                if lines.is_empty() {
                    continue;
                }
                let diagnostic = report(
                    self,
                    source,
                    item.node.start_byte(),
                    item.node.end_byte(),
                    format!("Use `receive_messages` instead of multiple stubs on lines {lines:?}."),
                    diagnostics,
                );
                owners.push((item.node.start_byte(), diagnostic));
            }
            if owners.is_empty() || corrections.is_none() {
                continue;
            }
            let Some((start, end, text)) = replacement(source, &siblings, &items) else {
                continue;
            };
            for (owner, index) in owners {
                corrections.as_deref_mut().unwrap().push(Correction {
                    start,
                    end,
                    replacement: text.clone(),
                    cop_name: self.name(),
                    cop_index: 0,
                    owner: Some(owner),
                });
                diagnostics[index].corrected = true;
            }
        }
    }
}
