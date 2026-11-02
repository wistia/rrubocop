use crate::cop::shared::ruby_expression::{
    integer_ast, plain_string, plain_symbol, regular_send_arguments, regular_send_name,
    regular_send_receiver, static_regex_value_and_options_for_ruby,
};
use crate::cop::shared::{
    call_method_name, call_receiver, collect_comments, method_node, named_kids, node_bytes,
    node_text, push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
fn literal(source: &SourceFile, node: Node<'_>) -> bool {
    node.kind() == "regex"
        || plain_string(source, node).is_some()
        || plain_symbol(source, node).is_some()
}
fn scope(node: Node<'_>) -> Option<Node<'_>> {
    let mut parent = node.parent();
    while let Some(node) = parent {
        if matches!(
            node.kind(),
            "method" | "singleton_method" | "class" | "module"
        ) {
            return Some(node);
        }
        parent = node.parent();
    }
    None
}
fn nodes(root: Node<'_>) -> Vec<Node<'_>> {
    let mut result = Vec::new();
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        result.push(node);
        let mut children = named_kids(node);
        if matches!(node.kind(), "if_modifier" | "unless_modifier") {
            if let Some(condition) = node.child_by_field_name("condition") {
                children.retain(|n| *n != condition);
                children.insert(0, condition);
            }
        }
        pending.extend(children.into_iter().rev());
    }
    result
}
fn candidate(source: &SourceFile, node: Node<'_>, config: &CopConfig) -> bool {
    let Some(method) = regular_send_name(source, node) else {
        return false;
    };
    let Some(args) = regular_send_arguments(source, node) else {
        return false;
    };
    let receiver = regular_send_receiver(source, node).flatten();
    match method.as_ref() {
        b"match" => {
            args.len() == 1
                && (receiver.is_some_and(|r| literal(source, r)) || literal(source, args[0]))
                || args.len() == 2 && integer_ast(source, args[1]).is_some()
        }
        b"=~" | b"!~" => {
            receiver.is_some() && args.len() == 1 && {
                if method.as_ref() == b"=~"
                    && node.kind() == "binary"
                    && receiver.is_some_and(|r| r.kind() == "regex")
                {
                    let regexp = receiver.unwrap();
                    crate::cop::ported::regexp_capture_counts(source, regexp)
                        .is_none_or(|(named, _)| named == 0)
                } else {
                    true
                }
            }
        }
        b"===" => {
            receiver.is_some_and(|r| {
                r.kind() == "regex"
                    && static_regex_value_and_options_for_ruby(
                        source,
                        r,
                        config.get_f64("TargetRubyVersion", 3.4),
                    )
                    .is_some_and(|(_, options)| options.len() <= 1)
            }) && args.len() == 1
        }
        _ => false,
    }
}
fn observer(source: &SourceFile, node: Node<'_>) -> bool {
    if node.kind() == "global_variable" {
        let mut target = node;
        let mut parent = target.parent();
        while parent.is_some_and(|n| {
            matches!(
                n.kind(),
                "left_assignment_list" | "rest_assignment" | "parenthesized_statements"
            )
        }) {
            target = parent.unwrap();
            parent = target.parent();
        }
        if parent.is_some_and(|parent| match parent.kind() {
            "assignment" | "operator_assignment" => {
                parent.child_by_field_name("left") == Some(target)
            }
            "for" => parent.child_by_field_name("pattern") == Some(target),
            "exception_variable" => true,
            _ => false,
        }) {
            return false;
        }
        let text = node_bytes(source, node);
        return matches!(
            text,
            b"$~"
                | b"$&"
                | b"$+"
                | b"$'"
                | b"$`"
                | b"$MATCH"
                | b"$PREMATCH"
                | b"$POSTMATCH"
                | b"$LAST_PAREN_MATCH"
                | b"$LAST_MATCH_INFO"
        ) || text.get(1).is_some_and(u8::is_ascii_digit);
    }
    if call_method_name(source, node) != Some(b"last_match")
        || crate::cop::shared::argument_nodes(node).len() > 1
        || node
            .child_by_field_name("operator")
            .is_some_and(|op| node_bytes(source, op) == b"&.")
    {
        return false;
    }
    call_receiver(node).is_some_and(|r| {
        r.kind() == "constant" && node_bytes(source, r) == b"Regexp"
            || r.kind() == "scope_resolution"
                && r.child_by_field_name("scope").is_none()
                && r.child_by_field_name("name")
                    .is_some_and(|n| node_bytes(source, n) == b"Regexp")
    })
}
fn start(node: Node<'_>) -> usize {
    node.parent()
        .filter(|p| matches!(p.kind(), "if_modifier" | "unless_modifier"))
        .and_then(|p| p.child_by_field_name("body"))
        .map_or(node.start_byte(), |n| n.start_byte())
}
fn observed(source: &SourceFile, node: Node<'_>, config: &CopConfig) -> bool {
    let scope = scope(node);
    let mut root = scope
        .and_then(|n| n.child_by_field_name("body"))
        .unwrap_or(node);
    if scope.is_none() {
        while let Some(parent) = root.parent() {
            root = parent;
        }
    }
    let nodes = nodes(root);
    let from = start(node);
    let to = nodes
        .iter()
        .find(|n| candidate(source, **n, config) && start(**n) > from && self::scope(**n) == scope)
        .map_or(usize::MAX, |n| n.start_byte());
    nodes.iter().any(|n| {
        n.start_byte() >= from
            && n.start_byte() <= to
            && self::scope(*n) == scope
            && observer(source, *n)
    })
}
fn condition(node: Node<'_>) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    if matches!(
        parent.kind(),
        "if" | "unless" | "elsif" | "conditional" | "if_modifier" | "unless_modifier"
    ) {
        return parent.child_by_field_name("condition") == Some(node);
    }
    let when = if parent.kind() == "pattern" {
        parent.parent()
    } else {
        Some(parent)
    };
    when.is_some_and(|when| {
        when.kind() == "when"
            && when.child_by_field_name("body") != Some(node)
            && when.parent().is_some_and(|case| {
                case.kind() == "case" && case.child_by_field_name("value").is_none()
            })
    })
}
fn indirect_captures_observed(source: &SourceFile, mut node: Node<'_>) -> bool {
    while let Some(parent) = node.parent() {
        node = parent;
    }
    nodes(node)
        .iter()
        .any(|n| crate::cop::shared::indirect_capture_observer(source, *n))
}
pub struct RegexpMatch;
impl Cop for RegexpMatch {
    fn name(&self) -> &'static str {
        "Performance/RegexpMatch"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "binary"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if config.get_f64("TargetRubyVersion", 3.4) < 2.4
            || node.child_by_field_name("block").is_some()
            || !condition(node)
            || !candidate(source, node, config)
            || observed(source, node, config)
        {
            return;
        }
        let selector = if node.kind() == "binary" {
            node.child_by_field_name("operator")
        } else {
            method_node(node)
        };
        let Some(selector) = selector else {
            return;
        };
        let method = node_text(source, selector);
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            format!("Use `match?` instead of `{method}` when `MatchData` is not used."),
        );
        let end = super::send_range::end(node);
        d.highlight_length = String::from_utf8_lossy(&source.as_bytes()[node.start_byte()..end])
            .split('\n')
            .next()
            .unwrap_or("")
            .chars()
            .count();
        d.correctable = !config.autocorrect_disabled();
        if d.correctable
            && collect_comments(node).is_empty()
            && !indirect_captures_observed(source, node)
        {
            let recv = regular_send_receiver(source, node).flatten();
            let args = regular_send_arguments(source, node).unwrap();
            if recv.is_some_and(|r| literal(source, r)) {
                if method == "match" {
                    d.corrected = push_replace(
                        &mut corrections,
                        selector.start_byte(),
                        selector.end_byte(),
                        "match?",
                        self.name(),
                    );
                } else if args.len() == 1 {
                    let recv = recv.unwrap();
                    let replacement = format!(
                        "{}{}.match?({})",
                        if method == "!~" { "!" } else { "" },
                        node_text(source, recv),
                        node_text(source, args[0])
                    );
                    let mut candidate = source.as_bytes().to_vec();
                    candidate.splice(node.start_byte()..end, replacement.bytes());
                    let candidate = SourceFile::from_bytes("candidate.rb", candidate);
                    if crate::parse::parse_ruby(&candidate)
                        .is_ok_and(|t| !t.root_node().has_error())
                    {
                        d.corrected = push_replace(
                            &mut corrections,
                            node.start_byte(),
                            end,
                            replacement,
                            self.name(),
                        );
                    }
                }
            }
        }
        diagnostics.push(d);
    }
}
