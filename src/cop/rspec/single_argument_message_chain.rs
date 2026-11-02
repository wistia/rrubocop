use crate::cop::rspec::helpers::RSPEC_INCLUDE;
use crate::cop::rspec::parity_helpers::*;
use crate::cop::shared::node_text;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

fn literal_argument(source: &SourceFile, node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "string"
            | "chained_string"
            | "subshell"
            | "integer"
            | "float"
            | "simple_symbol"
            | "delimited_symbol"
            | "true"
            | "false"
            | "nil"
            | "regex"
            | "range"
            | "heredoc_beginning"
    ) || node.kind() == "unary" && integer_value(source, node).is_some()
}
fn literal_contains_dot(source: &SourceFile, node: Node<'_>) -> bool {
    let mut pending = vec![node];
    while let Some(child) = pending.pop() {
        if child.kind() == "string_content" && node_text(source, child).contains('.')
            || child.kind() == "float"
            || crate::cop::ported::static_literal(source, child)
                .is_some_and(|value| value.contains('.'))
        {
            return true;
        }
        pending.extend(code_children(child));
    }
    false
}

pub struct SingleArgumentMessageChain;
impl Cop for SingleArgumentMessageChain {
    fn name(&self) -> &'static str {
        "RSpec/SingleArgumentMessageChain"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "identifier"]
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
        let _ = config;
        if safe_send(source, node) {
            return;
        }
        let Some(method) = name(source, node) else {
            return;
        };
        let replacement = match method {
            b"receive_message_chain" => "receive",
            b"stub_chain" => "stub",
            _ => return,
        };
        let args = call_args(node);
        if args.len() != 1 {
            return;
        }
        let argument = args[0];
        let pair = if argument.kind() == "pair" {
            Some(argument)
        } else if argument.kind() == "hash" {
            let pairs = code_children(argument);
            if pairs.len() != 1 || pairs[0].kind() != "pair" {
                return;
            }
            Some(pairs[0])
        } else {
            None
        };
        let array = argument.kind() == "array";
        if array && code_children(argument).len() != 1 {
            return;
        }
        if pair.is_none()
            && !array
            && (!literal_argument(source, argument) || literal_contains_dot(source, argument))
        {
            return;
        }
        let target = selector(node);
        let message = format!(
            "Use `{replacement}` instead of calling `{}` with a single argument.",
            String::from_utf8_lossy(method)
        );
        let index = report(
            self,
            source,
            target.start_byte(),
            target.end_byte(),
            message,
            diagnostics,
        );
        let mut edits = vec![(
            target.start_byte(),
            target.end_byte(),
            replacement.to_string(),
        )];
        if let Some(pair) = pair {
            let Some(key) = pair.child_by_field_name("key") else {
                return;
            };
            let Some(value) = pair.child_by_field_name("value") else {
                return;
            };
            let key = if symbol_node(key) || key.kind() == "hash_key_symbol" {
                crate::cop::ported::static_literal(source, key)
                    .map(|value| ruby_symbol(&value))
                    .unwrap_or_else(|| node_text(source, key))
            } else {
                node_text(source, key)
            };
            edits.push((argument.start_byte(), argument.end_byte(), key));
            edits.push((
                header_end(node),
                header_end(node),
                format!(".and_return({})", node_text(source, value)),
            ));
        } else if array {
            edits.push((
                argument.start_byte(),
                argument.end_byte(),
                node_text(source, code_children(argument)[0]),
            ));
        }
        let start = node.start_byte();
        let end = header_end(node);
        let mut text = node_text(source, node);
        text.truncate(end - start);
        edits.sort_by_key(|edit| std::cmp::Reverse(edit.0));
        for (first, last, replacement) in edits {
            text.replace_range(first - start..last - start, &replacement);
        }
        if preserving_replace(
            source,
            node,
            &mut corrections,
            start,
            end,
            text,
            self.name(),
        ) {
            diagnostics[index].corrected = true;
        }
    }
}
