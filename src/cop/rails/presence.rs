//! Preserve branch evaluation when suggesting ActiveSupport presence.
fn has_crlf(source: &SourceFile) -> bool {
    struct CrLf(bool);
    source
        .cached(0, || {
            CrLf(source.as_bytes().windows(2).any(|b| b == b"\r\n"))
        })
        .0
}
fn line_literal(source: &SourceFile, mut root: Node<'_>) -> bool {
    struct LineLiteral(bool);
    source
        .cached(0, || {
            while let Some(parent) = root.parent() {
                root = parent;
            }
            let mut found = false;
            crate::cop::shared::for_each_descendant(root, |node| {
                found |= crate::cop::shared::is_line_literal(source, node);
            });
            LineLiteral(found)
        })
        .0
}
use crate::cop::shared::ruby_expression::{
    plain_string, plain_symbol, regular_send_arguments, regular_send_name, regular_send_receiver,
    same_expression,
};
use crate::cop::shared::{
    collect_comments, method_node, named_kids, node_bytes, node_text, push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct Presence;
fn children(node: Node<'_>) -> Vec<Node<'_>> {
    named_kids(node)
        .into_iter()
        .filter(|n| {
            !matches!(
                n.kind(),
                "comment" | "empty_statement" | "heredoc_body" | "heredoc_end"
            )
        })
        .collect()
}
#[derive(Clone)]
enum Branch<'a> {
    Missing,
    One(Node<'a>),
    Sequence(Vec<Node<'a>>),
}
fn branch(node: Option<Node<'_>>) -> Branch<'_> {
    let Some(node) = node else {
        return Branch::Missing;
    };
    if matches!(
        node.kind(),
        "then" | "else" | "body_statement" | "block_body"
    ) {
        let kids = children(node);
        match kids.len() {
            0 => Branch::Missing,
            1 => Branch::One(kids[0]),
            _ => Branch::Sequence(kids),
        }
    } else {
        Branch::One(node)
    }
}
fn signed_same(
    source: &SourceFile,
    left: Node<'_>,
    left_negative: bool,
    mut right: Node<'_>,
    mut right_negative: bool,
) -> bool {
    if right.kind() == "unary" {
        if let Some(operand) = right.child_by_field_name("operand").filter(|n| {
            n.kind() == "call"
                && crate::cop::shared::numeric_send_start(source, *n) == right.start_byte()
        }) {
            right_negative ^= right
                .child_by_field_name("operator")
                .is_some_and(|n| node_bytes(source, n) == b"-");
            right = operand;
        }
    }
    if let Some(equal) = crate::cop::shared::ruby_expression::numeric_sign_equal(
        source,
        left,
        left_negative,
        right,
        right_negative,
    ) {
        return equal;
    }
    if left.kind() != "call" || right.kind() != "call" {
        return false;
    }
    let safe = |n: Node<'_>| {
        n.child_by_field_name("operator")
            .is_some_and(|op| node_bytes(source, op) == b"&.")
    };
    if safe(left) != safe(right)
        || crate::cop::shared::call_method_name(source, left)
            != crate::cop::shared::call_method_name(source, right)
        || !crate::cop::shared::ruby_expression::same_arguments(
            source,
            &crate::cop::shared::argument_nodes(left),
            &crate::cop::shared::argument_nodes(right),
        )
    {
        return false;
    }
    match (
        left.child_by_field_name("block"),
        right.child_by_field_name("block"),
    ) {
        (None, None) => {}
        (Some(a), Some(b)) if same_expression(source, a, b) => {}
        _ => return false,
    }
    match (
        crate::cop::shared::call_receiver(left),
        crate::cop::shared::call_receiver(right),
    ) {
        (Some(a), Some(b)) => signed_same(source, a, left_negative, b, right_negative),
        _ => false,
    }
}
fn matches_receiver(source: &SourceFile, receiver: Node<'_>, body: &Branch<'_>) -> bool {
    match body {
        Branch::One(node) => same_expression(source, receiver, *node),
        Branch::Sequence(nodes) if receiver.kind() == "parenthesized_statements" => {
            let recv = children(receiver);
            recv.len() == nodes.len()
                && recv
                    .iter()
                    .zip(nodes)
                    .all(|(a, b)| same_expression(source, *a, *b))
        }
        _ => false,
    }
}
fn ignored(other: &Branch<'_>) -> bool {
    match other {
        Branch::Sequence(_) => true,
        Branch::One(node) => {
            matches!(
                node.kind(),
                "parenthesized_statements"
                    | "if"
                    | "unless"
                    | "elsif"
                    | "conditional"
                    | "if_modifier"
                    | "unless_modifier"
                    | "rescue_modifier"
                    | "while"
            ) || (node.kind() == "while_modifier"
                && !node
                    .child_by_field_name("body")
                    .is_some_and(|n| n.kind() == "begin"))
        }
        _ => false,
    }
}
fn previous_truthy(node: Node<'_>) -> bool {
    let mut current = node;
    while let Some(parent) = current.parent() {
        let kids = children(parent);
        let index = kids.iter().position(|n| *n == current).unwrap_or(0);
        match parent.kind() {
            "program" | "body_statement" | "block_body" | "then" | "else" | "do" => {
                if kids.len() > 1 {
                    return index > 0;
                }
                if parent.kind() == "program" {
                    return false;
                }
                current = parent;
            }
            "argument_list" => {
                if index > 0 {
                    return true;
                }
                return parent.parent().is_some_and(|p| {
                    p.kind() == "call"
                        && !(p.child_by_field_name("receiver").is_none()
                            && method_node(p).is_some_and(|method| method.kind() == "super"))
                });
            }
            "method" | "singleton_method" | "module" | "singleton_class" | "block" | "do_block"
            | "lambda" => return true,
            "class" => return parent.child_by_field_name("superclass").is_some(),
            "assignment" | "operator_assignment" => {
                return parent.child_by_field_name("right") == Some(current);
            }
            "conditional" | "if" | "unless" | "elsif" => {
                if parent.child_by_field_name("condition") == Some(current) {
                    return false;
                }
                let swapped = parent.kind() == "unless";
                let first = parent.child_by_field_name(if swapped {
                    "alternative"
                } else {
                    "consequence"
                });
                if first == Some(current) {
                    return true;
                }
                return !matches!(branch(first), Branch::Missing);
            }
            "if_modifier" => return parent.child_by_field_name("body") == Some(current),
            "unless_modifier" | "rescue_modifier" => return false,
            "rescue" => return parent.child_by_field_name("variable").is_some(),
            "ensure" => {
                if kids.len() > 1 {
                    return index > 0;
                }
                return parent.parent().is_some_and(|owner| {
                    children(owner)
                        .iter()
                        .take_while(|n| **n != parent)
                        .next()
                        .is_some()
                });
            }
            "call" => return parent.child_by_field_name("receiver") != Some(current),
            _ => return index > 0,
        }
    }
    false
}
fn text(source: &SourceFile, node: Node<'_>) -> String {
    node_text(source, node).replace("\r\n", "\n")
}
fn compact(value: &str) -> String {
    value
        .split('\n')
        .map(|line| line.trim_start_matches(|c: char| c.is_ascii_whitespace()))
        .collect()
}
fn flattened(value: &str) -> String {
    let mut out = String::new();
    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\n' {
            out.push(' ');
            while chars.peek().is_some_and(|c| c.is_ascii_whitespace()) {
                chars.next();
            }
        } else {
            out.push(c);
        }
    }
    out
}
fn fallback(source: &SourceFile, other: &Branch<'_>) -> Option<String> {
    let Branch::One(node) = other else {
        return Some(String::new());
    };
    if node.kind() == "nil" {
        return Some(String::new());
    }
    let raw = text(source, *node);
    if let Some(command) = crate::cop::shared::command_unary_plus_call(source, *node) {
        let first = command.arguments.first()?;
        let method = String::from_utf8_lossy(&source.as_bytes()[command.start..first.start - 1]);
        let params = command
            .arguments
            .iter()
            .map(|arg| String::from_utf8_lossy(&source.as_bytes()[arg.start..arg.end]).into_owned())
            .collect::<Vec<_>>()
            .join(", ");
        return Some(format!(" || {method}({params})"));
    }
    if node.child_by_field_name("block").is_none() {
        if let Some(name) = regular_send_name(source, *node) {
            let args = regular_send_arguments(source, *node)
                .map(crate::cop::shared::ruby_arguments::from_nodes)
                .unwrap_or_default();
            let parentheses = node
                .child_by_field_name("arguments")
                .is_some_and(|n| node_bytes(source, n).starts_with(b"("));
            if !args.is_empty()
                && !parentheses
                && name.as_ref() != b"[]"
                && !matches!(name.as_ref(), b"+" | b"-" | b"*" | b"/" | b"%" | b"**")
            {
                let first = args[0].start();
                if first <= node.start_byte() {
                    return None;
                }
                let method =
                    String::from_utf8_lossy(&source.as_bytes()[node.start_byte()..first - 1]);
                let params = args
                    .iter()
                    .map(|arg| {
                        String::from_utf8_lossy(&source.as_bytes()[arg.start()..arg.end()])
                            .into_owned()
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                return Some(format!(" || {method}({params})"));
            }
        }
    }
    Some(format!(" || {raw}"))
}
fn fallback_safe(source: &SourceFile, other: &Branch<'_>) -> bool {
    let Branch::One(node) = other else {
        return true;
    };
    if matches!(
        node.kind(),
        "assignment"
            | "until_modifier"
            | "while_modifier"
            | "return"
            | "break"
            | "next"
            | "redo"
            | "retry"
    ) {
        return false;
    }
    if node.kind() == "binary"
        && node
            .child_by_field_name("operator")
            .is_some_and(|n| node_bytes(source, n) == b"and")
    {
        return false;
    }
    if crate::cop::shared::command_unary_plus_call(source, *node).is_some() {
        return true;
    }
    if let Some(name) = regular_send_name(source, *node) {
        let args = regular_send_arguments(source, *node).unwrap_or_default();
        let parentheses = node
            .child_by_field_name("arguments")
            .is_some_and(|n| node_bytes(source, n).starts_with(b"("));
        if let Some(first) = args.first().filter(|_| {
            !parentheses
                && !matches!(
                    name.as_ref(),
                    b"[]" | b"+" | b"-" | b"*" | b"/" | b"%" | b"**"
                )
        }) {
            return source
                .as_bytes()
                .get(first.start_byte().saturating_sub(1))
                .is_some_and(u8::is_ascii_whitespace);
        }
    }
    true
}
fn stable(source: &SourceFile, node: Node<'_>) -> bool {
    if node.kind() == "identifier" {
        return crate::cop::ported::local_identifier(source, node);
    }
    matches!(
        node.kind(),
        "self"
            | "instance_variable"
            | "class_variable"
            | "global_variable"
            | "integer"
            | "float"
            | "rational"
            | "complex"
            | "nil"
            | "true"
            | "false"
    ) || plain_symbol(source, node).is_some()
        || plain_string(source, node).is_some()
}
struct UnsafePresence(bool);
fn overridden(source: &SourceFile, node: Node<'_>) -> bool {
    source
        .cached(0, || {
            let mut root = node;
            while let Some(parent) = root.parent() {
                root = parent;
            }
            let mut stack = vec![root];
            while let Some(node) = stack.pop() {
                if matches!(node.kind(), "method" | "singleton_method")
                    && node.child_by_field_name("name").is_some_and(|n| {
                        matches!(
                            node_bytes(source, n),
                            b"presence" | b"present?" | b"blank?" | b"empty?" | b"respond_to?"
                        )
                    })
                {
                    return UnsafePresence(true);
                }
                if regular_send_name(source, node).is_some_and(|name| {
                    matches!(
                        name.as_ref(),
                        b"define_method"
                            | b"define_singleton_method"
                            | b"class_eval"
                            | b"module_eval"
                            | b"eval"
                            | b"prepend"
                    )
                }) {
                    return UnsafePresence(true);
                }
                stack.extend(named_kids(node));
            }
            UnsafePresence(false)
        })
        .0
}
impl Cop for Presence {
    fn name(&self) -> &'static str {
        "Rails/Presence"
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &[
            "conditional",
            "if",
            "unless",
            "if_modifier",
            "unless_modifier",
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
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(original_condition) = node.child_by_field_name("condition") else {
            return;
        };
        let mut condition = original_condition;
        let negative = regular_send_name(source, condition).as_deref() == Some(b"!")
            && regular_send_arguments(source, condition).is_some_and(|a| a.is_empty());
        if negative {
            let Some(Some(inner)) = regular_send_receiver(source, condition) else {
                return;
            };
            condition = inner;
        }
        let mut prefix = None;
        if condition.kind() == "unary" {
            if let Some(inner) = condition.child_by_field_name("operand").filter(|n| {
                n.kind() == "call"
                    && crate::cop::shared::numeric_send_start(source, *n) == condition.start_byte()
            }) {
                let minus = condition
                    .child_by_field_name("operator")
                    .is_some_and(|n| node_bytes(source, n) == b"-");
                prefix = Some((condition.start_byte(), minus));
                condition = inner;
            }
        }
        let Some(name) = regular_send_name(source, condition) else {
            return;
        };
        if !matches!(name.as_ref(), b"present?" | b"blank?")
            || !regular_send_arguments(source, condition).is_some_and(|a| a.is_empty())
        {
            return;
        }
        let Some(Some(receiver)) = regular_send_receiver(source, condition) else {
            return;
        };
        let (mut yes, mut no) = if matches!(node.kind(), "if_modifier" | "unless_modifier") {
            (branch(node.child_by_field_name("body")), Branch::Missing)
        } else {
            (
                branch(node.child_by_field_name("consequence")),
                branch(node.child_by_field_name("alternative")),
            )
        };
        if matches!(node.kind(), "unless" | "unless_modifier") {
            std::mem::swap(&mut yes, &mut no);
        }
        let (mirror, other) = if (name.as_ref() == b"present?") != negative {
            (&yes, &no)
        } else {
            (&no, &yes)
        };
        let mirrored = if let Some((_, minus)) = prefix {
            match mirror {
                Branch::One(other) => signed_same(source, receiver, minus, *other, false),
                _ => false,
            }
        } else {
            matches_receiver(source, receiver, mirror)
        };
        if !mirrored || ignored(other) {
            return;
        }
        let Some(suffix) = fallback(source, other) else {
            return;
        };
        let receiver_text = prefix.map_or_else(
            || text(source, receiver),
            |(start, _)| {
                String::from_utf8_lossy(&source.as_bytes()[start..receiver.end_byte()])
                    .replace("\r\n", "\n")
            },
        );
        let replacement = format!("{receiver_text}.presence{suffix}");
        let replacement = if previous_truthy(node) {
            format!("({replacement})")
        } else {
            replacement
        };
        let raw = text(source, node);
        let current = if node.kind() != "conditional" && raw.contains('\n') {
            let display_condition = if prefix.is_some() && !negative {
                condition
            } else {
                original_condition
            };
            let end = method_node(display_condition)
                .or_else(|| display_condition.child_by_field_name("operator"))
                .map_or(display_condition.end_byte(), |n| n.end_byte());
            format!(
                "{} ... end",
                String::from_utf8_lossy(&source.as_bytes()[node.start_byte()..end])
                    .replace("\r\n", "\n")
            )
        } else {
            flattened(&raw)
        };
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            format!(
                "Use `{}` instead of `{}`.",
                compact(&replacement),
                compact(&current)
            ),
        );
        d.highlight_length =
            super::blank::highlight_length(source, node.start_byte(), node.end_byte());
        d.correctable = !config.autocorrect_disabled()
            && stable(source, receiver)
            && fallback_safe(source, other)
            && !overridden(source, node)
            && collect_comments(node).is_empty()
            && !has_crlf(source);
        if d.correctable {
            let removed_lines = raw.bytes().filter(|b| *b == b'\n').count()
                != replacement.bytes().filter(|b| *b == b'\n').count();
            d.correctable = !(removed_lines && line_literal(source, node));
            let mut proposed = source.as_bytes().to_vec();
            proposed.splice(node.byte_range(), replacement.bytes());
            let proposed = SourceFile::from_bytes("presence.rb", proposed);
            d.correctable &=
                crate::parse::parse_ruby(&proposed).is_ok_and(|tree| !tree.root_node().has_error());
        }
        if d.correctable {
            d.corrected = push_replace(
                &mut corrections,
                node.start_byte(),
                node.end_byte(),
                replacement,
                self.name(),
            );
        }
        diagnostics.push(d);
    }
}
