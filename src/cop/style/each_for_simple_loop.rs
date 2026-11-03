//! Style/EachForSimpleLoop.

use tree_sitter::Node;

use crate::cop::shared::{
    call_method_name, call_receiver, integer_value, node_bytes, push_replace,
    semantic_argument_nodes,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EachForSimpleLoop;

fn implicit_parameter(
    source: &SourceFile,
    node: Node<'_>,
    matches_name: fn(&[u8]) -> bool,
) -> bool {
    if matches!(
        node.kind(),
        "block"
            | "do_block"
            | "lambda"
            | "method"
            | "singleton_method"
            | "class"
            | "module"
            | "singleton_class"
    ) {
        return false;
    }
    if node.kind() == "identifier"
        && matches_name(node_bytes(source, node))
        && node.parent().is_none_or(|parent| {
            parent.child_by_field_name("method") != Some(node)
                && parent.child_by_field_name("name") != Some(node)
        })
    {
        return true;
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .any(|child| implicit_parameter(source, child, matches_name))
}

fn value_observed(node: Node<'_>) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    if parent.kind() == "program" {
        return false;
    }
    if matches!(parent.kind(), "body_statement" | "block_body") {
        let mut cursor = parent.walk();
        return parent
            .named_children(&mut cursor)
            .filter(|child| child.kind() != "comment")
            .last()
            == Some(node);
    }
    if parent.kind() == "parenthesized_statements" {
        let mut cursor = parent.walk();
        if parent
            .named_children(&mut cursor)
            .filter(|child| child.kind() != "comment")
            .last()
            != Some(node)
        {
            return false;
        }
        return value_observed(parent);
    }
    true
}

impl Cop for EachForSimpleLoop {
    fn name(&self) -> &'static str {
        "Style/EachForSimpleLoop"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"each"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if call_method_name(source, node) != Some(b"each")
            || !semantic_argument_nodes(node).is_empty()
        {
            return;
        }
        let Some(block) = node.child_by_field_name("block") else {
            return;
        };
        if block
            .child_by_field_name("parameters")
            .is_some_and(|parameters| {
                let mut cursor = parameters.walk();
                parameters
                    .named_children(&mut cursor)
                    .any(|child| child.kind() != "comment")
            })
        {
            return;
        }
        if block.child_by_field_name("body").is_some_and(|body| {
            implicit_parameter(source, body, |name| matches!(name, [b'_', b'1'..=b'9']))
        }) {
            return;
        }
        let Some(receiver) =
            call_receiver(node).filter(|receiver| receiver.kind() == "parenthesized_statements")
        else {
            return;
        };
        let mut cursor = receiver.walk();
        let ranges: Vec<_> = receiver
            .named_children(&mut cursor)
            .filter(|child| child.kind() != "comment")
            .collect();
        if ranges.len() != 1 || ranges[0].kind() != "range" {
            return;
        }
        let range = ranges[0];
        let Some(first) = range
            .child_by_field_name("begin")
            .and_then(|first| integer_value(source, first))
        else {
            return;
        };
        let Some(last) = range
            .child_by_field_name("end")
            .and_then(|last| integer_value(source, last))
        else {
            return;
        };
        let inclusive = range
            .child_by_field_name("operator")
            .is_some_and(|operator| node_bytes(source, operator) == b"..");
        let implicit_it = first != 0.into()
            && config.get_f64("TargetRubyVersion", 3.4) >= 3.4
            && block
                .child_by_field_name("body")
                .is_some_and(|body| implicit_parameter(source, body, |name| name == b"it"));
        let count = last - first + num_bigint::BigInt::from(u8::from(inclusive));
        let Some(method) = node.child_by_field_name("method") else {
            return;
        };
        let end = node
            .child_by_field_name("arguments")
            .map_or(method.end_byte(), |arguments| arguments.end_byte());
        let (line, column) = source.offset_to_line_col(node.start_byte());
        let mut diagnostic = self.diagnostic(
            source,
            line,
            column,
            "Use `Integer#times` for a simple loop which iterates a fixed number of times.".into(),
        );
        diagnostic.highlight_length =
            String::from_utf8_lossy(&source.as_bytes()[node.start_byte()..end])
                .chars()
                .count();
        let removes_comment = crate::cop::shared::collect_comments(node)
            .iter()
            .any(|comment| comment.end_byte() <= end);
        if value_observed(node) || removes_comment || implicit_it {
            diagnostic.correctable = false;
        } else {
            diagnostic.corrected = push_replace(
                &mut corrections,
                node.start_byte(),
                end,
                format!("{count}.times"),
                self.name(),
            );
        }
        diagnostics.push(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_ruby_3_4_implicit_it_values() {
        let diagnostics =
            crate::testutil::run_cop_full(&EachForSimpleLoop, b"(3..7).each { puts it }\n");
        assert_eq!(diagnostics.len(), 1);
        assert!(!diagnostics[0].correctable);
    }

    #[test]
    fn original_ruby_specs() {
        crate::testutil::assert_original_spec_parity(
            &EachForSimpleLoop,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/cops/style/each_for_simple_loop/parity.json"
            )),
        );
    }
    #[test]
    fn preserves_observed_range_return_value() {
        let diagnostics =
            crate::testutil::run_cop_full(&EachForSimpleLoop, b"result = (1..5).each {}\n");
        assert_eq!(diagnostics.len(), 1);
        assert!(!diagnostics[0].correctable);
    }
}
