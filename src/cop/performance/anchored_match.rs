use crate::cop::shared::ruby_literal::string_literal;
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, captures_observed, collect_comments,
    is_setter_call, node_text, push_replace, send_end,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

fn boolean_context(source: &SourceFile, node: Node<'_>) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    match parent.kind() {
        "if" | "unless" | "while" | "until" | "if_modifier" | "unless_modifier"
        | "while_modifier" | "until_modifier" | "conditional" => {
            parent.child_by_field_name("condition") == Some(node)
        }
        "unary" => parent
            .child_by_field_name("operator")
            .is_some_and(|n| matches!(node_text(source, n).as_str(), "!" | "not")),
        "binary"
            if parent.child_by_field_name("operator").is_some_and(|n| {
                matches!(node_text(source, n).as_str(), "&&" | "||" | "and" | "or")
            }) =>
        {
            boolean_context(source, parent)
        }
        "parenthesized_statements" => {
            parent.named_child_count() == 1 && boolean_context(source, parent)
        }
        _ => false,
    }
}
fn valid_replacement(receiver: &str, dot: &str, name: &str, literal: &str) -> Option<String> {
    let replacement = format!("{receiver}{dot}{name}({literal})");
    let source = SourceFile::from_bytes("match.rb", replacement.as_bytes().to_vec());
    let tree = crate::parse::parse_ruby(&source).ok()?;
    let root = tree.root_node();
    let call = root.named_child(0)?;
    if root.has_error()
        || root.named_child_count() != 1
        || call.kind() != "call"
        || call_method_name(&source, call) != Some(name.as_bytes())
        || node_text(&source, call_receiver(call)?) != receiver
    {
        return None;
    }
    Some(replacement)
}
fn diagnostic(
    cop: &dyn Cop,
    suffix: bool,
    source: &SourceFile,
    node: Node<'_>,
    end: usize,
) -> Diagnostic {
    let name = if suffix { "end_with?" } else { "start_with?" };
    let side = if suffix { "end" } else { "beginning" };
    let (line, col) = source.offset_to_line_col(node.start_byte());
    let mut d = cop.diagnostic(
        source,
        line,
        col,
        format!(
            "Use `String#{name}` instead of a regex match anchored to the {side} of the string."
        ),
    );
    let bytes = &source.as_bytes()[node.start_byte()..end];
    let line_end = bytes
        .iter()
        .position(|b| *b == b'\n')
        .unwrap_or(bytes.len());
    let line = &bytes[..line_end];
    let line = if line_end < bytes.len() {
        line.strip_suffix(b"\r").unwrap_or(line)
    } else {
        line
    };
    d.highlight_length = String::from_utf8_lossy(line).chars().count();
    d
}
pub(super) fn check(
    cop: &dyn Cop,
    suffix: bool,
    source: &SourceFile,
    node: Node<'_>,
    config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    mut corrections: Option<&mut Vec<Correction>>,
) {
    let (left, right, method, dot, end) = if node.kind() == "binary" {
        if !node
            .child_by_field_name("operator")
            .is_some_and(|n| node_text(source, n) == "=~")
        {
            return;
        }
        let (Some(left), Some(right)) = (
            node.child_by_field_name("left"),
            node.child_by_field_name("right"),
        ) else {
            return;
        };
        (
            left,
            right,
            "=~".to_string(),
            ".".to_string(),
            node.end_byte(),
        )
    } else {
        if is_setter_call(node) {
            return;
        }
        let Some(method) = call_method_name(source, node) else {
            return;
        };
        if !matches!(method, b"match" | b"match?" | b"=~") {
            return;
        }
        let Some(left) = call_receiver(node) else {
            return;
        };
        let args = argument_nodes(node);
        let dot = node
            .child_by_field_name("operator")
            .map(|n| node_text(source, n))
            .unwrap_or_else(|| ".".into());
        if args.is_empty()
            && suffix
            && dot != "&."
            && super::anchored_regexp::read(source, left, suffix, config).is_some()
        {
            let mut d = diagnostic(cop, suffix, source, node, send_end(node));
            d.correctable = false;
            diagnostics.push(d);
            return;
        }
        if args.len() != 1 {
            return;
        }
        (
            left,
            args[0],
            String::from_utf8_lossy(method).into_owned(),
            dot,
            send_end(node),
        )
    };
    let (receiver, pattern) =
        if let Some(pattern) = super::anchored_regexp::read(source, right, suffix, config) {
            (left, pattern)
        } else if dot != "&." {
            let Some(pattern) = super::anchored_regexp::read(source, left, suffix, config) else {
                return;
            };
            (right, pattern)
        } else {
            return;
        };
    let name = if suffix { "end_with?" } else { "start_with?" };
    let mut d = diagnostic(cop, suffix, source, node, end);
    let replacement = pattern.text.as_deref().and_then(|text| {
        valid_replacement(
            &node_text(source, receiver),
            &dot,
            name,
            &string_literal(text),
        )
    });
    d.correctable = !config.autocorrect_disabled()
        && node.child_by_field_name("block").is_none()
        && replacement.is_some()
        && pattern.absolute
        && (method == "match?" || !captures_observed(source, node))
        && collect_comments(node).is_empty()
        && (method == "match?" || boolean_context(source, node))
        && !matches!(
            receiver.kind(),
            "nil"
                | "regex"
                | "array"
                | "hash"
                | "symbol"
                | "simple_symbol"
                | "delimited_symbol"
                | "integer"
                | "float"
                | "true"
                | "false"
        );
    if d.correctable {
        d.corrected = push_replace(
            &mut corrections,
            node.start_byte(),
            end,
            replacement.unwrap(),
            cop.name(),
        );
    }
    diagnostics.push(d);
}
