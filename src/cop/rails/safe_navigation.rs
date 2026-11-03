//! Preserve eager argument evaluation and public dispatch when replacing ActiveSupport try calls.
use crate::cop::shared::ruby_arguments::{Argument, arguments};
use crate::cop::shared::ruby_expression::{
    ShorthandValue, plain_string, plain_symbol, regular_send_name, regular_send_receiver,
    shorthand_value,
};
use crate::cop::shared::{
    collect_comments, named_kids, node_bytes, node_text, numeric_send_start, push_replace, send_end,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct SafeNavigation;
struct DispatchPolicy {
    blocked: bool,
    line: bool,
}
fn dispatch_policy(source: &SourceFile, node: Node<'_>) -> std::sync::Arc<DispatchPolicy> {
    source.cached(0, || {
        let mut root = node;
        while let Some(parent) = root.parent() {
            root = parent;
        }
        let mut stack = vec![root];
        let mut policy = DispatchPolicy {
            blocked: false,
            line: false,
        };
        while let Some(node) = stack.pop() {
            policy.line |= crate::cop::shared::is_line_literal(source, node);
            if matches!(node.kind(), "method" | "singleton_method")
                && node.child_by_field_name("name").is_some_and(|name| {
                    matches!(
                        node_bytes(source, name),
                        b"try" | b"try!" | b"public_send" | b"respond_to?"
                    )
                })
            {
                policy.blocked = true;
            }
            if regular_send_name(source, node).is_some_and(|name| {
                matches!(
                    name.as_ref(),
                    b"private"
                        | b"protected"
                        | b"private_class_method"
                        | b"define_method"
                        | b"define_singleton_method"
                        | b"class_eval"
                        | b"module_eval"
                        | b"eval"
                        | b"prepend"
                )
            }) {
                policy.blocked = true;
            }
            stack.extend(named_kids(node));
        }
        policy
    })
}
fn non_nil(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "integer"
            | "float"
            | "rational"
            | "complex"
            | "string"
            | "chained_string"
            | "array"
            | "hash"
            | "simple_symbol"
            | "delimited_symbol"
            | "true"
            | "false"
            | "character"
    )
}
fn passive(source: &SourceFile, node: Node<'_>) -> bool {
    if matches!(
        node.kind(),
        "integer" | "float" | "rational" | "complex" | "true" | "false" | "nil"
    ) || plain_string(source, node).is_some()
        || plain_symbol(source, node).is_some()
    {
        return node.kind() != "heredoc_beginning";
    }
    if node.kind() == "identifier" {
        return crate::cop::ported::local_identifier(source, node);
    }
    if matches!(node.kind(), "array" | "hash") {
        return named_kids(node)
            .into_iter()
            .filter(|n| n.kind() != "comment")
            .all(|n| passive(source, n));
    }
    if node.kind() == "pair" {
        let Some(key) = node.child_by_field_name("key") else {
            return false;
        };
        if let Some(role) = shorthand_value(source, key) {
            return role == ShorthandValue::Local;
        }
        return (plain_string(source, key).is_some()
            || plain_symbol(source, key).is_some()
            || matches!(key.kind(), "integer" | "float" | "true" | "false" | "nil"))
            && node
                .child_by_field_name("value")
                .is_some_and(|n| passive(source, n));
    }
    false
}
fn passive_argument(source: &SourceFile, arg: &Argument<'_>) -> bool {
    arg.nodes.iter().all(|node| passive(source, *node))
}
fn try_method_exists(receiver: Node<'_>, method: &str) -> bool {
    if receiver.kind() == "nil" {
        return true;
    }
    match receiver.kind() {
        "array" => matches!(
            method,
            "join" | "map" | "each_with_object" | "length" | "size" | "empty?" | "to_s"
        ),
        "string" | "chained_string" | "character" => matches!(
            method,
            "upcase" | "downcase" | "strip" | "size" | "length" | "empty?" | "to_s"
        ),
        "hash" => matches!(
            method,
            "keys" | "values" | "size" | "length" | "empty?" | "to_s"
        ),
        "integer" | "float" => matches!(method, "abs" | "zero?" | "to_i" | "to_f" | "to_s"),
        "true" | "false" => matches!(method, "to_s" | "itself" | "nil?"),
        _ => false,
    }
}
impl Cop for SafeNavigation {
    fn name(&self) -> &'static str {
        "Rails/SafeNavigation"
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"try", b"try!"]
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if config.get_f64("TargetRubyVersion", 3.4) < 2.3 {
            return;
        }
        let Some(name) = regular_send_name(source, node) else {
            return;
        };
        if name.as_ref() != b"try!"
            && !(name.as_ref() == b"try" && config.get_bool("ConvertTry", false))
        {
            return;
        }
        let args = arguments(node);
        let Some(dispatch) = args.first().and_then(Argument::node) else {
            return;
        };
        let Some(method) = plain_symbol(source, dispatch) else {
            return;
        };
        if !method
            .bytes()
            .any(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            return;
        }
        let Some(receiver) = regular_send_receiver(source, node) else {
            return;
        };
        let start = numeric_send_start(source, node);
        let end = send_end(node);
        let (line, col) = source.offset_to_line_col(start);
        let mut d = self.diagnostic(
            source,
            line,
            col,
            format!(
                "Use safe navigation (`&.`) instead of `{}`.",
                String::from_utf8_lossy(&name)
            ),
        );
        d.highlight_length = super::blank::highlight_length(source, start, end);
        d.correctable = false;
        let params = &args[1..];
        let setter = method.ends_with('=');
        let standard_symbol = node_text(source, dispatch) == format!(":{method}");
        let receiver_allowed = receiver.is_some_and(|r| r.kind() != "self")
            && (name.as_ref() == b"try!"
                || receiver.is_some_and(|r| try_method_exists(r, &method)));
        let eager_safe =
            receiver.is_some_and(non_nil) || params.iter().all(|arg| passive_argument(source, arg));
        let setter_safe = !setter
            || (params.len() == 1
                && node.child_by_field_name("block").is_none()
                && !crate::cop::shared::ruby_semantics::value_used(source, node));
        let policy = dispatch_policy(source, node);
        if !config.autocorrect_disabled()
            && standard_symbol
            && receiver_allowed
            && eager_safe
            && setter_safe
            && !policy.blocked
        {
            let from = node
                .child_by_field_name("operator")
                .map_or(node.start_byte(), |n| n.start_byte());
            let comments = collect_comments(node);
            let comment = comments
                .iter()
                .any(|n| n.start_byte() < end && n.end_byte() > from);
            let mut pending = vec![node];
            let mut heredoc = false;
            while let Some(n) = pending.pop() {
                if n.start_byte() >= end {
                    continue;
                }
                if n.kind() == "heredoc_beginning" {
                    heredoc = true;
                }
                pending.extend(named_kids(n));
            }
            let joined = params
                .iter()
                .map(|arg| {
                    String::from_utf8_lossy(&source.as_bytes()[arg.start()..arg.end()]).into_owned()
                })
                .collect::<Vec<_>>()
                .join(", ");
            let replacement = if setter {
                format!("&.{} = {joined}", &method[..method.len() - 1])
            } else if params.is_empty() {
                format!("&.{method}")
            } else {
                format!("&.{method}({joined})")
            };
            let has_lines = source.as_bytes()[from..end]
                .iter()
                .filter(|b| **b == b'\n')
                .count()
                != replacement.bytes().filter(|b| *b == b'\n').count();
            let line_sensitive = policy.line && has_lines;
            if !comment
                && !heredoc
                && !line_sensitive
                && !source.as_bytes().windows(2).any(|b| b == b"\r\n")
            {
                let mut proposed = source.as_bytes().to_vec();
                proposed.splice(from..end, replacement.bytes());
                let proposed = SourceFile::from_bytes("safe.rb", proposed);
                d.correctable = crate::parse::parse_ruby(&proposed)
                    .is_ok_and(|tree| !tree.root_node().has_error());
                if d.correctable {
                    d.corrected =
                        push_replace(&mut corrections, from, end, replacement, self.name());
                }
            }
        }
        diagnostics.push(d);
    }
}
