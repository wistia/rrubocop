use crate::cop::shared::ruby_expression::{
    integer_ast, plain_string, plain_symbol, regular_send_arguments, regular_send_name,
    regular_send_receiver, same_arguments,
};
use crate::cop::shared::{
    call_method_name, collect_comments, method_node, named_kids, node_bytes, node_text,
    push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
fn expression(mut node: Node<'_>) -> Option<Node<'_>> {
    if matches!(node.kind(), "body_statement" | "block_body") {
        let children = named_kids(node)
            .into_iter()
            .filter(|n| n.kind() != "comment")
            .collect::<Vec<_>>();
        if children.len() != 1 {
            return None;
        }
        node = children[0];
    }
    Some(node)
}
fn literal(source: &SourceFile, node: Node<'_>) -> bool {
    match node.kind() {
        "array" | "hash" | "pair" => named_kids(node).iter().all(|n| literal(source, *n)),
        "true" | "false" | "nil" | "hash_key_symbol" => true,
        _ => {
            integer_ast(source, node).is_some()
                || plain_string(source, node).is_some()
                || plain_symbol(source, node).is_some()
        }
    }
}
pub struct CompareWithBlock;
impl Cop for CompareWithBlock {
    fn name(&self) -> &'static str {
        "Performance/CompareWithBlock"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"sort", b"sort!", b"max", b"min", b"minmax"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(method) = call_method_name(source, node) else {
            return;
        };
        let replacement = match method {
            b"sort" => "sort_by",
            b"sort!" => "sort_by!",
            b"min" => "min_by",
            b"max" => "max_by",
            b"minmax" => "minmax_by",
            _ => return,
        };
        if !regular_send_arguments(source, node).is_some_and(|args| args.is_empty()) {
            return;
        }
        let Some(block) = node.child_by_field_name("block") else {
            return;
        };
        let Some(parameters) = block.child_by_field_name("parameters") else {
            return;
        };
        let params = named_kids(parameters)
            .into_iter()
            .filter(|n| n.kind() != "comment")
            .collect::<Vec<_>>();
        if params.len() != 2 || params.iter().any(|p| p.kind() != "identifier") {
            return;
        }
        let Some(body) = block.child_by_field_name("body").and_then(expression) else {
            return;
        };
        if regular_send_name(source, body).as_deref() != Some(b"<=>") {
            return;
        }
        let Some(Some(left)) = regular_send_receiver(source, body) else {
            return;
        };
        let Some(rights) = regular_send_arguments(source, body) else {
            return;
        };
        if rights.len() != 1 {
            return;
        }
        let right = rights[0];
        if left.child_by_field_name("block").is_some()
            || right.child_by_field_name("block").is_some()
        {
            return;
        }
        let (Some(Some(a)), Some(Some(b))) = (
            regular_send_receiver(source, left),
            regular_send_receiver(source, right),
        ) else {
            return;
        };
        if a.kind() != "identifier"
            || b.kind() != "identifier"
            || !crate::cop::ported::local_identifier(source, a)
            || !crate::cop::ported::local_identifier(source, b)
            || node_bytes(source, a) != node_bytes(source, params[0])
            || node_bytes(source, b) != node_bytes(source, params[1])
        {
            return;
        }
        let (Some(projection), Some(other)) = (
            regular_send_name(source, left),
            regular_send_name(source, right),
        ) else {
            return;
        };
        if projection != other {
            return;
        }
        let (Some(args), Some(other_args)) = (
            regular_send_arguments(source, left),
            regular_send_arguments(source, right),
        ) else {
            return;
        };
        if !same_arguments(source, &args, &other_args) {
            return;
        }
        let indexed = projection.as_ref() == b"[]";
        if indexed {
            if args.len() != 1
                || !(integer_ast(source, args[0]).is_some()
                    || plain_string(source, args[0]).is_some()
                    || plain_symbol(source, args[0]).is_some())
            {
                return;
            }
        } else if !args.is_empty() {
            return;
        }
        let a = node_text(source, params[0]);
        let b = node_text(source, params[1]);
        let (instead, left_text, right_text) = if indexed {
            let key = node_text(source, args[0]);
            (
                format!(" {{ |a| a[{key}] }}"),
                format!("{a}[{key}]"),
                format!("{b}[{key}]"),
            )
        } else {
            let projection = String::from_utf8_lossy(&projection);
            (
                format!("(&:{projection})"),
                format!("{a}.{projection}"),
                format!("{b}.{projection}"),
            )
        };
        let Some(selector) = method_node(node) else {
            return;
        };
        let (line, col) = source.offset_to_line_col(selector.start_byte());
        let mut d=self.diagnostic(source,line,col,format!("Use `{replacement}{instead}` instead of `{} {{ |{a}, {b}| {left_text} <=> {right_text} }}`.",String::from_utf8_lossy(method)));
        d.highlight_length =
            String::from_utf8_lossy(&source.as_bytes()[selector.start_byte()..block.end_byte()])
                .split('\n')
                .next()
                .unwrap_or("")
                .chars()
                .count();
        d.correctable = !config.autocorrect_disabled();
        if d.correctable
            && indexed
            && collect_comments(block).is_empty()
            && regular_send_receiver(source, node)
                .flatten()
                .is_some_and(|r| r.kind() == "array" && literal(source, r))
        {
            d.corrected = push_replace(
                &mut corrections,
                selector.start_byte(),
                block.end_byte(),
                format!("{replacement}{instead}"),
                self.name(),
            );
        }
        diagnostics.push(d);
    }
}
