use crate::cop::shared::ruby_expression::{
    integer_ast, regular_send_arguments, regular_send_receiver,
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

pub struct Detect;
impl Cop for Detect {
    fn name(&self) -> &'static str {
        "Performance/Detect"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn safe_autocorrect(&self) -> bool {
        false
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "element_reference"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let indexed = node.kind() == "element_reference";
        let method = if indexed {
            b"[]".as_slice()
        } else {
            let Some(method) = call_method_name(source, node) else {
                return;
            };
            method
        };
        if !matches!(method, b"first" | b"last" | b"[]") {
            return;
        }
        if node
            .child_by_field_name("operator")
            .is_some_and(|op| node_bytes(source, op) == b"&.")
        {
            return;
        }
        let Some(args) = regular_send_arguments(source, node) else {
            return;
        };
        let reverse = if indexed {
            if args.len() != 1 {
                return;
            }
            let Some(index) = integer_ast(source, args[0]) else {
                return;
            };
            match index.to_string().as_str() {
                "0" => false,
                "-1" => true,
                _ => return,
            }
        } else {
            if !args.is_empty() {
                return;
            }
            method == b"last"
        };
        let Some(Some(filter)) = regular_send_receiver(source, node) else {
            return;
        };
        let Some(first) = call_method_name(source, filter) else {
            return;
        };
        if !matches!(first, b"select" | b"find_all" | b"filter") {
            return;
        }
        if indexed
            && filter
                .child_by_field_name("operator")
                .is_some_and(|op| node_bytes(source, op) == b"&.")
        {
            return;
        }
        let Some(filter_args) = regular_send_arguments(source, filter).or_else(|| {
            // The original accepts csend for first/last, but not index lookup.
            (!indexed).then(|| crate::cop::shared::argument_nodes(filter))
        }) else {
            return;
        };
        let block = filter.child_by_field_name("block");
        if let Some(block) = block {
            if config.get_f64("TargetRubyVersion", 3.4) >= 2.7
                && crate::cop::shared::ruby_expression::numbered_bindings(source, block).is_some()
            {
                return;
            }
            let Some(body) = block.child_by_field_name("body") else {
                return;
            };
            if named_kids(body)
                .into_iter()
                .all(|n| matches!(n.kind(), "comment" | "empty_statement"))
            {
                return;
            }
            if !filter_args.is_empty() {
                return;
            }
        } else if !filter_args
            .first()
            .is_some_and(|arg| arg.kind() == "block_argument")
        {
            return;
        }
        if call_receiver(filter).is_some_and(|receiver| {
            call_method_name(source, receiver) == Some(b"lazy") && call_receiver(receiver).is_some()
        }) {
            return;
        }
        let Some(first_selector) = method_node(filter) else {
            return;
        };
        let selector_end = if indexed {
            node.end_byte()
        } else {
            let Some(selector) = method_node(node) else {
                return;
            };
            selector.end_byte()
        };
        let preferred = match config.options.get("PreferredDetectMethod") {
            Some(value) => value.as_str().unwrap_or("detect"),
            None => "find",
        };
        let replacement = format!("{}{preferred}", if reverse { "reverse." } else { "" });
        let suffix = if indexed {
            format!(
                "{}[{}]",
                String::from_utf8_lossy(first),
                if reverse { -1 } else { 0 }
            )
        } else {
            format!(
                "{}.{}",
                String::from_utf8_lossy(first),
                String::from_utf8_lossy(method)
            )
        };
        let (line, col) = source.offset_to_line_col(first_selector.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            format!("Use `{replacement}` instead of `{suffix}`."),
        );
        d.highlight_length =
            String::from_utf8_lossy(&source.as_bytes()[first_selector.start_byte()..selector_end])
                .split('\n')
                .next()
                .unwrap_or("")
                .chars()
                .count();
        d.correctable = !config.autocorrect_disabled();
        let safe = call_receiver(filter).is_some_and(|r| {
            r.kind() == "array"
                && named_kids(r)
                    .iter()
                    .all(|n| matches!(n.kind(), "integer" | "true" | "false" | "nil"))
        }) && block
            .and_then(|b| b.child_by_field_name("body"))
            .is_some_and(|body| matches!(node_text(source, body).trim(), "true" | "false" | "nil"))
            && collect_comments(node).is_empty();
        if d.correctable && safe {
            let mut candidate = source.as_bytes().to_vec();
            candidate.splice(filter.end_byte()..selector_end, [].iter().copied());
            candidate.splice(
                first_selector.start_byte()..first_selector.end_byte(),
                replacement.bytes(),
            );
            let candidate = SourceFile::from_bytes("candidate.rb", candidate);
            if crate::parse::parse_ruby(&candidate).is_ok_and(|tree| !tree.root_node().has_error())
            {
                d.corrected = push_replace(
                    &mut corrections,
                    first_selector.start_byte(),
                    first_selector.end_byte(),
                    replacement,
                    self.name(),
                );
                d.corrected |= push_replace(
                    &mut corrections,
                    filter.end_byte(),
                    selector_end,
                    String::new(),
                    self.name(),
                );
            }
        }
        diagnostics.push(d);
    }
}
