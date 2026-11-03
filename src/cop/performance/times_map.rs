//! Build arrays only when the literal iteration count preserves Integer#times behavior.
use crate::cop::shared::ruby_expression::{float_value, integer_ast};
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, collect_comments, named_kids, node_bytes,
    numeric_send_start, push_replace, send_end,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub struct TimesMap;
struct TimesPolicy {
    blocked: bool,
    line: bool,
}
fn policy(source: &SourceFile, node: Node<'_>) -> std::sync::Arc<TimesPolicy> {
    source.cached(0, || {
        let mut root = node;
        while let Some(parent) = root.parent() {
            root = parent;
        }
        let mut stack = vec![root];
        let mut result = TimesPolicy {
            blocked: false,
            line: false,
        };
        while let Some(node) = stack.pop() {
            result.line |= crate::cop::shared::is_line_literal(source, node);
            if matches!(node.kind(), "method" | "singleton_method")
                && node.child_by_field_name("name").is_some_and(|n| {
                    matches!(
                        node_bytes(source, n),
                        b"times" | b"map" | b"collect" | b"new" | b"method" | b"to_proc"
                    )
                })
            {
                result.blocked = true;
            }
            let constant = match node.kind() {
                "assignment" | "operator_assignment" => node.child_by_field_name("left"),
                "class" | "module" => node.child_by_field_name("name"),
                _ => None,
            };
            if constant.is_some_and(|n| {
                n.kind() == "constant" && node_bytes(source, n) == b"Array"
                    || n.kind() == "scope_resolution"
                        && n.child_by_field_name("name")
                            .is_some_and(|n| node_bytes(source, n) == b"Array")
            }) {
                result.blocked = true;
            }
            if matches!(
                call_method_name(source, node),
                Some(
                    b"define_method"
                        | b"define_singleton_method"
                        | b"class_eval"
                        | b"module_eval"
                        | b"eval"
                        | b"prepend"
                )
            ) {
                result.blocked = true;
            }
            stack.extend(named_kids(node));
        }
        result
    })
}
fn literal(source: &SourceFile, node: Node<'_>) -> bool {
    integer_ast(source, node).is_some()
        || float_value(source, node).is_some()
        || matches!(
            node.kind(),
            "string"
                | "chained_string"
                | "character"
                | "simple_symbol"
                | "delimited_symbol"
                | "array"
                | "string_array"
                | "symbol_array"
                | "hash"
                | "regex"
                | "range"
                | "rational"
                | "complex"
                | "true"
                | "false"
                | "nil"
                | "subshell"
                | "heredoc_beginning"
        )
        || node.kind() == "identifier" && node_bytes(source, node) == b"__FILE__"
}
fn non_nil_block(source: &SourceFile, argument: Node<'_>) -> bool {
    let Some(value) = argument.named_child(0) else {
        return false;
    };
    matches!(
        value.kind(),
        "simple_symbol" | "delimited_symbol" | "lambda"
    ) || value.kind() == "call"
        && call_receiver(value).is_none()
        && call_method_name(source, value) == Some(b"method")
}
fn owns_block(source: &SourceFile, node: Node<'_>, block: Node<'_>) -> bool {
    if block.kind() != "do_block" {
        return true;
    }
    let mut root = node;
    while let Some(parent) = root
        .parent()
        .filter(|parent| parent.kind() == "call" && call_receiver(*parent) == Some(root))
    {
        root = parent;
    }
    !root
        .parent()
        .and_then(|parent| crate::cop::shared::command_unary_plus(source, parent))
        .is_some_and(|command| command.right == root)
}
impl Cop for TimesMap {
    fn name(&self) -> &'static str {
        "Performance/TimesMap"
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"map", b"collect"]
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
        if crate::cop::shared::is_setter_call(node) {
            return;
        }
        let Some(method @ (b"map" | b"collect")) = call_method_name(source, node) else {
            return;
        };
        let Some(times) = call_receiver(node).filter(|n| {
            n.kind() == "call"
                && n.child_by_field_name("block").is_none()
                && call_method_name(source, *n) == Some(b"times")
                && argument_nodes(*n).is_empty()
        }) else {
            return;
        };
        let Some(count) = call_receiver(times) else {
            return;
        };
        let args = argument_nodes(node);
        let block = node.child_by_field_name("block");
        if block.is_some_and(|block| !owns_block(source, node, block)) {
            return;
        }
        if !(block.is_some() && args.is_empty()
            || block.is_none() && args.len() == 1 && args[0].kind() == "block_argument")
        {
            return;
        }
        let numeric = integer_ast(source, count).is_some() || float_value(source, count).is_some();
        if !numeric
            && !times
                .child_by_field_name("operator")
                .is_some_and(|n| node_bytes(source, n) == b".")
        {
            return;
        }
        let start = numeric_send_start(source, node);
        let count_source = String::from_utf8_lossy(&source.as_bytes()[start..count.end_byte()])
            .replace("\r\n", "\n");
        let suffix = if literal(source, count) {
            String::new()
        } else {
            format!(" only if `{count_source}` is always 0 or more")
        };
        let (line, col) = source.offset_to_line_col(start);
        let mut d = self.diagnostic(
            source,
            line,
            col,
            format!(
                "Use `Array.new({count_source})` with a block instead of `.times.{}`{suffix}.",
                String::from_utf8_lossy(method)
            ),
        );
        let highlighted = &source.as_bytes()[start..node.end_byte()];
        let end = highlighted
            .iter()
            .position(|byte| *byte == b'\n')
            .unwrap_or(highlighted.len());
        let first_line = if highlighted.get(end) == Some(&b'\n')
            && highlighted.get(end.wrapping_sub(1)) == Some(&b'\r')
        {
            &highlighted[..end - 1]
        } else {
            &highlighted[..end]
        };
        d.highlight_length = String::from_utf8_lossy(first_line).chars().count();
        let mut value = integer_ast(source, count);
        if start < node.start_byte() && source.as_bytes()[start] == b'-' {
            value = value.map(|v| -v);
        }
        let policy = policy(source, node);
        d.correctable = !config.autocorrect_disabled()
            && value.is_some_and(|v| {
                v.to_string().parse::<usize>().is_ok_and(|length| {
                    length <= (isize::MAX as usize) / std::mem::size_of::<usize>()
                })
            })
            && !policy.blocked
            && (block.is_some()
                || args
                    .first()
                    .is_some_and(|argument| non_nil_block(source, *argument)));
        let header = send_end(node);
        if d.correctable {
            let extra = args
                .iter()
                .map(|arg| {
                    format!(
                        ", {}",
                        String::from_utf8_lossy(&source.as_bytes()[arg.byte_range()])
                    )
                })
                .collect::<String>();
            let replacement = format!("Array.new({count_source}{extra})");
            let comments = collect_comments(node)
                .iter()
                .any(|n| n.start_byte() < header);
            let lines = source.as_bytes()[start..header]
                .iter()
                .filter(|b| **b == b'\n')
                .count()
                != replacement.bytes().filter(|b| *b == b'\n').count();
            d.correctable = !comments
                && !(policy.line && lines)
                && !source.as_bytes().windows(2).any(|b| b == b"\r\n");
            let mut proposed = source.as_bytes().to_vec();
            proposed.splice(start..header, replacement.bytes());
            let proposed = SourceFile::from_bytes("times.rb", proposed);
            d.correctable &=
                crate::parse::parse_ruby(&proposed).is_ok_and(|tree| !tree.root_node().has_error());
            if d.correctable {
                d.corrected =
                    push_replace(&mut corrections, start, header, replacement, self.name());
            }
        }
        diagnostics.push(d);
    }
}
