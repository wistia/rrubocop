//! Resolve filter action names against direct lexical definitions and one-step aliases.
use crate::cop::shared::ruby_expression::{plain_string, plain_symbol};
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, named_kids, node_text, send_end, word_value,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use std::collections::HashMap;
use tree_sitter::Node;
pub struct LexicallyScopedActionFilter;
const METHODS: &[&[u8]] = &[
    b"after_action",
    b"append_after_action",
    b"append_around_action",
    b"append_before_action",
    b"around_action",
    b"before_action",
    b"prepend_after_action",
    b"prepend_around_action",
    b"prepend_before_action",
    b"skip_after_action",
    b"skip_around_action",
    b"skip_before_action",
    b"skip_action_callback",
];
fn children(node: Node<'_>) -> Vec<Node<'_>> {
    named_kids(node)
        .into_iter()
        .filter(|n| n.kind() != "comment")
        .collect()
}
fn scalar(source: &SourceFile, node: Node<'_>) -> Option<String> {
    plain_symbol(source, node).or_else(|| plain_string(source, node))
}
fn values(source: &SourceFile, node: Node<'_>) -> Vec<String> {
    if matches!(node.kind(), "string_array" | "symbol_array") {
        return children(node)
            .into_iter()
            .filter_map(|n| word_value(source, n, node))
            .collect();
    }
    if node.kind() == "array" {
        return children(node)
            .into_iter()
            .filter_map(|n| scalar(source, n))
            .collect();
    }
    scalar(source, node).into_iter().collect()
}
fn defined(source: &SourceFile, parent: Node<'_>) -> Vec<String> {
    let Some(body) = parent.child_by_field_name("body") else {
        return Vec::new();
    };
    let statements = if let Some(superclass) = parent
        .child_by_field_name("superclass")
        .and_then(|n| children(n).first().copied())
        .filter(|n| n.kind() == "parenthesized_statements")
    {
        children(superclass)
    } else {
        let statements = children(body);
        if statements.len() == 1 && statements[0].kind() == "parenthesized_statements" {
            children(statements[0])
        } else if statements.len() >= 2 {
            statements
        } else {
            return Vec::new();
        }
    };
    let mut methods: Vec<_> = statements
        .iter()
        .filter(|n| n.kind() == "method")
        .filter_map(|n| n.child_by_field_name("name"))
        .map(|n| node_text(source, n))
        .collect();
    let mut aliases = HashMap::new();
    for node in statements {
        let pair = if node.kind() == "alias" {
            node.child_by_field_name("alias")
                .zip(node.child_by_field_name("name"))
                .map(|(old, new)| {
                    (
                        scalar(source, old).unwrap_or_else(|| node_text(source, old)),
                        Some(scalar(source, new).unwrap_or_else(|| node_text(source, new))),
                    )
                })
        } else if super::unknown_env::regular(node, source)
            && call_method_name(source, node) == Some(b"alias_method")
        {
            let args = argument_nodes(node);
            args.first().zip(args.last()).and_then(|(new, old)| {
                plain_symbol(source, *old).map(|old| (old, plain_symbol(source, *new)))
            })
        } else {
            None
        };
        if let Some((old, new)) = pair {
            aliases.insert(old, new);
        }
    }
    let aliased: Vec<_> = methods
        .iter()
        .filter_map(|name| aliases.get(name).cloned().flatten())
        .collect();
    methods.extend(aliased);
    methods
}
impl Cop for LexicallyScopedActionFilter {
    fn name(&self) -> &'static str {
        "Rails/LexicallyScopedActionFilter"
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        METHODS
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if !super::unknown_env::regular(node, source)
            || call_receiver(node).is_some()
            || crate::cop::shared::is_setter_call(node)
            || !call_method_name(source, node).is_some_and(|method| METHODS.contains(&method))
        {
            return;
        }
        let args = argument_nodes(node);
        let pairs = if args.len() == 2 && args[1].kind() == "hash" {
            children(args[1])
        } else if args.len() == 2 && args[1].kind() == "pair" {
            vec![args[1]]
        } else {
            return;
        };
        if pairs.len() != 1 {
            return;
        }
        let pair = pairs[0];
        if pair.kind() != "pair"
            || !pair
                .child_by_field_name("key")
                .and_then(|n| plain_symbol(source, n))
                .is_some_and(|name| matches!(name.as_str(), "only" | "except"))
        {
            return;
        }
        let Some(value) = pair.child_by_field_name("value") else {
            return;
        };
        let mut parent = node.parent();
        while parent.is_some_and(|n| !matches!(n.kind(), "class" | "module")) {
            parent = parent.and_then(|n| n.parent());
        }
        let Some(parent) = parent else {
            return;
        };
        let methods = defined(source, parent);
        let unmatched: Vec<_> = values(source, value)
            .into_iter()
            .filter(|name| !methods.contains(name))
            .collect();
        if unmatched.is_empty() {
            return;
        }
        let actions = if unmatched.len() == 1 {
            format!("`{}` is", unmatched[0])
        } else {
            format!("`{}` are", unmatched.join("`, `"))
        };
        let (line, col) = source.offset_to_line_col(node.start_byte());
        let mut d = self.diagnostic(
            source,
            line,
            col,
            format!("{actions} not explicitly defined on the {}.", parent.kind()),
        );
        d.highlight_length =
            super::blank::highlight_length(source, node.start_byte(), send_end(node));
        diagnostics.push(d);
    }
}
