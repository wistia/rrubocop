use crate::cop::shared::ruby_literal::string_literal;
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, captures_observed, collect_comments,
    is_const_named, is_setter_call, method_node, node_text, push_replace, send_end,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub(super) fn check(
    cop: &dyn Cop,
    suffix: bool,
    source: &SourceFile,
    node: Node<'_>,
    config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    mut corrections: Option<&mut Vec<Correction>>,
) {
    if config.get_f64("TargetRubyVersion", 3.4) < 2.5 || is_setter_call(node) {
        return;
    }
    let Some(method) = call_method_name(source, node) else {
        return;
    };
    if !matches!(method, b"gsub" | b"gsub!" | b"sub" | b"sub!") {
        return;
    }
    let Some(receiver) = call_receiver(node) else {
        return;
    };
    let args = argument_nodes(node);
    if args.len() != 2
        || args[0].kind() != "regex"
        || !matches!(args[1].kind(), "string" | "character" | "heredoc_beginning")
    {
        return;
    }
    if !crate::cop::ported::ruby_string_parts(source, args[1])
        .is_some_and(|p| p.len() == 1 && p[0].is_empty())
    {
        return;
    }
    let Some(pattern) = super::anchored_regexp::read(source, args[0], suffix, config) else {
        return;
    };
    let Some(selector) = method_node(node) else {
        return;
    };
    let name = if suffix {
        "delete_suffix"
    } else {
        "delete_prefix"
    };
    let name = format!("{name}{}", if method.ends_with(b"!") { "!" } else { "" });
    let (line, col) = source.offset_to_line_col(selector.start_byte());
    let mut d = cop.diagnostic(
        source,
        line,
        col,
        format!(
            "Use `{name}` instead of `{}`.",
            String::from_utf8_lossy(method)
        ),
    );
    d.highlight_length = node_text(source, selector).chars().count();
    let known_pathname = receiver.kind() == "call"
        && (call_method_name(source, receiver) == Some(b"new")
            && call_receiver(receiver).is_some_and(|n| is_const_named(source, n, b"Pathname"))
            || call_method_name(source, receiver) == Some(b"Pathname")
                && call_receiver(receiver).is_none());
    let end = send_end(node);
    d.correctable = !config.autocorrect_disabled()
        && !known_pathname
        && pattern.text.is_some()
        && !captures_observed(source, node)
        && collect_comments(node).is_empty()
        && args[1].kind() != "heredoc_beginning"
        && pattern.absolute;
    if d.correctable {
        let dot = node
            .child_by_field_name("operator")
            .map(|n| node_text(source, n))
            .unwrap_or_else(|| ".".into());
        let replacement = format!(
            "{}{dot}{name}({})",
            node_text(source, receiver),
            string_literal(pattern.text.as_deref().unwrap())
        );
        d.corrected = push_replace(
            &mut corrections,
            node.start_byte(),
            end,
            replacement,
            cop.name(),
        );
    }
    diagnostics.push(d);
}
