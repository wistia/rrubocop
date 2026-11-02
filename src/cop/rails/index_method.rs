use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, collect_comments, for_each_descendant,
    method_node, named_kids, node_text, push_replace, send_end,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
fn children(node: Node<'_>) -> Vec<Node<'_>> {
    named_kids(node)
        .into_iter()
        .filter(|n| n.kind() != "comment")
        .collect()
}
fn local(source: &SourceFile, node: Node<'_>, name: &str) -> bool {
    node.kind() == "identifier"
        && node_text(source, node) == name
        && crate::cop::ported::local_identifier(source, node)
}
fn body(block: Node<'_>) -> Option<Node<'_>> {
    let mut node = block.child_by_field_name("body")?;
    while matches!(node.kind(), "block_body" | "body_statement" | "program") {
        let items = children(node);
        if items.len() != 1 {
            return None;
        }
        node = items[0];
    }
    Some(node)
}
struct Match<'a> {
    call: Node<'a>,
    parameters: Node<'a>,
    body: Node<'a>,
    expression: Node<'a>,
    element: Node<'a>,
}
fn matched<'a>(source: &SourceFile, call: Node<'a>, by: bool, each: bool) -> Option<Match<'a>> {
    let block = call.child_by_field_name("block")?;
    let parameters = block.child_by_field_name("parameters")?;
    let params = children(parameters);
    if params.len() != if each { 2 } else { 1 } || params.iter().any(|n| n.kind() != "identifier") {
        return None;
    }
    let name = node_text(source, params[0]);
    let body = body(block)?;
    let expression = if each {
        let memo = node_text(source, params[1]);
        if body.kind() != "assignment" {
            return None;
        }
        let left = body.child_by_field_name("left")?;
        let right = body.child_by_field_name("right")?;
        let (receiver, args) = if left.kind() == "element_reference" {
            let receiver = left.child_by_field_name("object")?;
            (
                receiver,
                children(left)
                    .into_iter()
                    .filter(|n| *n != receiver)
                    .collect::<Vec<_>>(),
            )
        } else if call_method_name(source, left) == Some(b"[]") {
            (call_receiver(left)?, argument_nodes(left))
        } else {
            return None;
        };
        if !local(source, receiver, &memo) || args.len() != 1 {
            return None;
        }
        let expression = if by {
            if !local(source, right, &name) {
                return None;
            }
            args[0]
        } else {
            if !local(source, args[0], &name) {
                return None;
            }
            right
        };
        let mut reads_memo = false;
        for_each_descendant(expression, |n| reads_memo |= local(source, n, &memo));
        if reads_memo {
            return None;
        }
        expression
    } else {
        if body.kind() != "array" {
            return None;
        }
        let items = children(body);
        if items.len() != 2 || !local(source, items[usize::from(by)], &name) {
            return None;
        }
        items[usize::from(!by)]
    };
    if local(source, expression, &name) {
        return None;
    }
    Some(Match {
        call,
        parameters,
        body,
        expression,
        element: params[0],
    })
}
pub(super) fn check(
    cop: &dyn Cop,
    source: &SourceFile,
    node: Node<'_>,
    config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    mut corrections: Option<&mut Vec<Correction>>,
    by: bool,
) {
    if crate::cop::shared::is_setter_call(node) {
        return;
    }
    if !by
        && (!config.get_bool("__RailtiesInLockfile", true)
            || config.get_f64(
                "__RailtiesVersion",
                config.get_f64("TargetRailsVersion", 8.1),
            ) < 6.0)
    {
        return;
    }
    let method = call_method_name(source, node);
    let args = argument_nodes(node);
    let mut matches = Vec::new();
    if method == Some(b"each_with_object")
        && args.len() == 1
        && args[0].kind() == "hash"
        && children(args[0]).is_empty()
    {
        if let Some(m) = matched(source, node, by, true) {
            matches.push((
                m,
                "each_with_object",
                node.start_byte(),
                node.end_byte(),
                false,
            ));
        }
    }
    if method == Some(b"to_h") && args.is_empty() && config.get_f64("TargetRubyVersion", 3.4) >= 2.6
    {
        if let Some(m) = matched(source, node, by, false) {
            matches.push((m, "to_h { ... }", node.start_byte(), node.end_byte(), false));
        }
    }
    if method == Some(b"to_h") && args.is_empty() {
        if let Some(map) = call_receiver(node).filter(|n| {
            matches!(call_method_name(source, *n), Some(b"map" | b"collect"))
                && argument_nodes(*n).is_empty()
        }) {
            if let Some(m) = matched(source, map, by, false) {
                matches.push((
                    m,
                    "map { ... }.to_h",
                    node.start_byte(),
                    send_end(node),
                    node.child_by_field_name("block").is_some(),
                ));
            }
        }
    }
    let hash = if node.kind() == "element_reference" {
        node.child_by_field_name("object").map(|receiver| {
            (
                receiver,
                children(node)
                    .into_iter()
                    .filter(|n| *n != receiver)
                    .collect::<Vec<_>>(),
            )
        })
    } else if method == Some(b"[]") && super::unknown_env::regular(node, source) {
        call_receiver(node).map(|receiver| (receiver, args))
    } else {
        None
    };
    if let Some((receiver, args)) = hash.filter(|(receiver, args)| {
        args.len() == 1
            && matches!(receiver.kind(), "constant" | "scope_resolution")
            && (receiver.kind() == "constant" && node_text(source, *receiver) == "Hash"
                || receiver.kind() == "scope_resolution"
                    && receiver.child_by_field_name("scope").is_none()
                    && receiver
                        .child_by_field_name("name")
                        .is_some_and(|name| node_text(source, name) == "Hash"))
    }) {
        let _ = receiver;
        let map = args[0];
        if matches!(call_method_name(source, map), Some(b"map" | b"collect"))
            && argument_nodes(map).is_empty()
        {
            if let Some(m) = matched(source, map, by, false) {
                matches.push((
                    m,
                    "Hash[map { ... }]",
                    node.start_byte(),
                    send_end(node),
                    false,
                ));
            }
        }
    }
    for (m, description, start, end, keep_suffix) in matches {
        let method = if by { "index_by" } else { "index_with" };
        let (line, col) = source.offset_to_line_col(start);
        let mut d = cop.diagnostic(
            source,
            line,
            col,
            format!("Prefer `{method}` over `{description}`."),
        );
        d.highlight_length = super::blank::highlight_length(source, start, end);
        let mut flow = false;
        for_each_descendant(m.expression, |n| {
            flow |= matches!(n.kind(), "break" | "next" | "redo" | "retry")
        });
        let selector = method_node(m.call);
        d.correctable = !config.autocorrect_disabled() && !flow && selector.is_some();
        if let Some(selector) = selector.filter(|_| d.correctable) {
            let selector_end = m
                .call
                .child_by_field_name("arguments")
                .map_or(selector.end_byte(), |n| n.end_byte());
            let mut edits = vec![
                (selector.start_byte(), selector_end, method.to_owned()),
                (
                    m.parameters.start_byte(),
                    m.parameters.end_byte(),
                    format!("|{}|", node_text(source, m.element)),
                ),
                (
                    m.body.start_byte(),
                    m.body.end_byte(),
                    node_text(source, m.expression),
                ),
            ];
            if start < m.call.start_byte() {
                edits.push((start, m.call.start_byte(), String::new()));
            }
            if end > m.call.end_byte() && !keep_suffix {
                edits.push((m.call.end_byte(), end, String::new()));
            }
            let lost_comment = collect_comments(node).iter().any(|comment| {
                edits.iter().any(|(start, end, _)| {
                    *start < comment.end_byte() && *end > comment.start_byte()
                }) && !(m.expression.start_byte() <= comment.start_byte()
                    && comment.end_byte() <= m.expression.end_byte())
            });
            let mut proposed = source.as_bytes().to_vec();
            edits.sort_by_key(|(start, _, _)| std::cmp::Reverse(*start));
            for (start, end, text) in &edits {
                proposed.splice(*start..*end, text.bytes());
            }
            let proposed = SourceFile::from_bytes("index.rb", proposed);
            d.correctable = !lost_comment
                && crate::parse::parse_ruby(&proposed)
                    .is_ok_and(|tree| !tree.root_node().has_error());
            if d.correctable {
                for (start, end, text) in edits {
                    d.corrected |= push_replace(&mut corrections, start, end, text, cop.name());
                }
            }
        }
        diagnostics.push(d);
    }
}
