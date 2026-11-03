//! Require explicit inverse associations when original Rails inference is unavailable.
use crate::cop::shared::ruby_expression::{numbered_bindings, plain_symbol};
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, method_node, named_kids, node_text,
};
use crate::cop::{Cop, CopConfig};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct InverseOf;
fn block(node: Node<'_>) -> Option<Node<'_>> {
    if node.kind() == "lambda" {
        Some(node)
    } else {
        node.child_by_field_name("block")
    }
}
fn ordinary_block(source: &SourceFile, node: Node<'_>, config: &CopConfig) -> bool {
    block(node).is_some()
        && (config.get_f64("TargetRubyVersion", 3.4) < 2.7
            || block(node)
                .and_then(|b| numbered_bindings(source, b))
                .is_none())
}
fn options<'a>(arguments: &[Node<'a>]) -> Vec<Node<'a>> {
    arguments
        .iter()
        .flat_map(|n| {
            if n.kind() == "hash" {
                named_kids(*n)
            } else if n.kind() == "pair" {
                vec![*n]
            } else {
                Vec::new()
            }
        })
        .filter(|n| n.kind() == "pair")
        .collect()
}
fn matches_option(source: &SourceFile, pair: Node<'_>, key: &str, nil: bool) -> bool {
    pair.child_by_field_name("key")
        .is_some_and(|n| plain_symbol(source, n).as_deref() == Some(key))
        && pair
            .child_by_field_name("value")
            .is_some_and(|n| n.kind() == "nil")
            == nil
}
fn same_context(source: &SourceFile, block: Node<'_>, recv: Option<Node<'_>>) -> bool {
    let first = block.child_by_field_name("parameters").and_then(|p| {
        named_kids(p)
            .into_iter()
            .find(|n| n.kind() != "block_local_variable")
    });
    match (first, recv) {
        (None, None) => true,
        (Some(arg), Some(recv)) => {
            let name = if arg.kind() == "identifier" {
                Some(arg)
            } else {
                arg.child_by_field_name("name")
            };
            name.is_some_and(|name| {
                recv.kind() == "identifier"
                    && crate::cop::ported::local_identifier(source, recv)
                    && node_text(source, name) == node_text(source, recv)
            })
        }
        _ => false,
    }
}
impl Cop for InverseOf {
    fn name(&self) -> &'static str {
        "Rails/InverseOf"
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"has_many", b"has_one", b"belongs_to"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        _corrections: Option<&mut Vec<crate::correction::Correction>>,
    ) {
        if !super::unknown_env::regular(node, source)
            || crate::cop::shared::is_setter_call(node)
            || !matches!(
                call_method_name(source, node),
                Some(b"has_many" | b"has_one" | b"belongs_to")
            )
        {
            return;
        }
        let arguments = argument_nodes(node);
        let Some(first) = arguments.first() else {
            return;
        };
        let arguments = if matches!(first.kind(), "pair" | "hash_splat_argument") {
            Vec::new()
        } else {
            arguments[1..].to_vec()
        };
        let mut opts = options(&arguments);
        let mut parent = node.parent();
        while let Some(ancestor) = parent {
            if call_method_name(source, ancestor) == Some(b"with_options")
                && call_receiver(ancestor).is_none()
                && ordinary_block(source, ancestor, config)
                && block(ancestor).is_some_and(|b| {
                    b.start_byte() <= node.start_byte()
                        && node.end_byte() <= b.end_byte()
                        && same_context(source, b, call_receiver(node))
                })
            {
                opts.extend(options(&argument_nodes(ancestor)));
            }
            parent = ancestor.parent();
        }
        let has = |key| opts.iter().any(|n| matches_option(source, *n, key, false));
        if has("through") || has("polymorphic") {
            return;
        }
        let scoped = !config.get_bool("IgnoreScopes", false)
            && arguments.iter().any(|n| ordinary_block(source, *n, config));
        if !scoped
            && !has("conditions")
            && !has("foreign_key")
            && !(config.get_f64("TargetRailsVersion", 8.1) < 5.2 && has("as"))
        {
            return;
        }
        if has("inverse_of") {
            return;
        }
        let message = if opts
            .iter()
            .any(|n| matches_option(source, *n, "inverse_of", true))
        {
            "You specified `inverse_of: nil`, you probably meant to use `inverse_of: false`."
        } else {
            "Specify an `:inverse_of` option."
        };
        let Some(method) = method_node(node) else {
            return;
        };
        let (line, col) = source.offset_to_line_col(method.start_byte());
        let mut d = self.diagnostic(source, line, col, message.into());
        d.highlight_length = node_text(source, method).chars().count();
        diagnostics.push(d);
    }
}
