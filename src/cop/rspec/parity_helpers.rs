use crate::cop::Cop;
use crate::cop::shared::{call_method_name, call_receiver, method_node, node_bytes};
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

pub(super) fn name<'a>(source: &'a SourceFile, node: Node<'_>) -> Option<&'a [u8]> {
    if node.kind() == "identifier" {
        if node.parent().is_some_and(|parent| {
            ["method", "name"]
                .iter()
                .any(|field| parent.child_by_field_name(field) == Some(node))
                || parent.kind() == "simple_symbol"
                || parent.kind() == "assignment" && parent.child_by_field_name("left") == Some(node)
        }) {
            return None;
        }
        (!local_read(source, node)).then(|| node_bytes(source, node))
    } else {
        call_method_name(source, node)
    }
}

pub(super) fn bare(source: &SourceFile, node: Node<'_>, expected: &[u8]) -> bool {
    call_receiver(node).is_none() && name(source, node) == Some(expected)
}
pub(super) fn runner(source: &SourceFile, node: Node<'_>) -> bool {
    name(source, node).is_some_and(|n| [b"to".as_slice(), b"not_to", b"to_not"].contains(&n))
}
pub(super) fn symbol<'a>(source: &'a SourceFile, node: Node<'_>) -> Option<&'a [u8]> {
    match node.kind() {
        "simple_symbol" | "symbol" => node_bytes(source, node).strip_prefix(b":"),
        "delimited_symbol" => {
            let quoted = node_bytes(source, node).strip_prefix(b":")?;
            if quoted.len() < 2 || quoted.windows(2).any(|bytes| bytes == b"#{") {
                return None;
            }
            Some(&quoted[1..quoted.len() - 1])
        }
        "string" if !node_bytes(source, node).windows(2).any(|b| b == b"#{") => {
            let bytes = node_bytes(source, node);
            (bytes.len() >= 2).then(|| &bytes[1..bytes.len() - 1])
        }
        _ => None,
    }
}
pub(super) fn header_end(node: Node<'_>) -> usize {
    node.child_by_field_name("arguments").map_or_else(
        || method_node(node).unwrap_or(node).end_byte(),
        |args| args.end_byte(),
    )
}

pub(super) fn report(
    cop: &dyn Cop,
    source: &SourceFile,
    start: usize,
    end: usize,
    message: String,
    diagnostics: &mut Vec<Diagnostic>,
) -> usize {
    let (line, column) = source.offset_to_line_col(start);
    let mut diagnostic = cop.diagnostic(source, line, column, message);
    diagnostic.highlight_length = if source.offset_to_line_col(end).0 == line {
        source
            .offset_to_line_col(end)
            .1
            .saturating_sub(column)
            .max(1)
    } else {
        source
            .line_start(line + 1)
            .unwrap_or(source.as_bytes().len())
            .saturating_sub(start + 1)
            .max(1)
    };
    diagnostics.push(diagnostic);
    diagnostics.len() - 1
}
pub(super) fn selector(node: Node<'_>) -> Node<'_> {
    method_node(node).unwrap_or(node)
}

fn contains_binding(source: &SourceFile, root: Node<'_>, target: Node<'_>, binding: &[u8]) -> bool {
    if root.start_byte() >= target.start_byte() {
        return false;
    }
    if matches!(
        root.kind(),
        "method" | "singleton_method" | "class" | "module" | "block" | "do_block"
    ) && !(root.start_byte() <= target.start_byte() && target.end_byte() <= root.end_byte())
    {
        return false;
    }
    if matches!(root.kind(), "assignment" | "operator_assignment") {
        if let Some(left) = root.child_by_field_name("left") {
            if left.kind() == "identifier" && node_bytes(source, left) == binding {
                return true;
            }
        }
    }
    let mut cursor = root.walk();
    root.named_children(&mut cursor)
        .any(|child| contains_binding(source, child, target, binding))
}

fn local_read(source: &SourceFile, node: Node<'_>) -> bool {
    let binding = node_bytes(source, node);
    if ![
        b"be".as_slice(),
        b"double",
        b"spy",
        b"is_expected",
        b"expect",
        b"receive_message_chain",
        b"stub_chain",
        b"raise_error",
        b"raise_exception",
        b"contain_exactly",
        b"should",
        b"should_not",
    ]
    .contains(&binding)
    {
        return false;
    }
    let mut current = node.parent();
    while let Some(scope) = current {
        if matches!(
            scope.kind(),
            "method" | "singleton_method" | "block" | "do_block" | "lambda"
        ) {
            if let Some(params) = scope.child_by_field_name("parameters") {
                let mut pending = vec![params];
                while let Some(parameter) = pending.pop() {
                    if parameter.kind() == "identifier" && node_bytes(source, parameter) == binding
                    {
                        return true;
                    }
                    let mut cursor = parameter.walk();
                    pending.extend(parameter.named_children(&mut cursor));
                }
            }
        }
        if matches!(
            scope.kind(),
            "method" | "singleton_method" | "class" | "module" | "program"
        ) {
            return contains_binding(source, scope, node, binding);
        }
        current = scope.parent();
    }
    false
}

pub(super) fn hooks() -> &'static [&'static [u8]] {
    &[
        b"before",
        b"after",
        b"around",
        b"prepend_before",
        b"append_before",
        b"prepend_after",
        b"append_after",
    ]
}
pub(super) fn expectations() -> &'static [&'static [u8]] {
    &[b"expect", b"expect_any_instance_of", b"is_expected"]
}
pub(super) fn metadata_call(source: &SourceFile, node: Node<'_>) -> bool {
    use crate::cop::rspec::helpers::{bare_rspec_call, call_block, is_example, is_group};
    let Some(method) = name(source, node) else {
        return false;
    };
    if !(is_example(method) || is_group(method) || hooks().contains(&method)) {
        return false;
    }
    if bare_rspec_call(source, node).is_some() && call_block(node).is_some() {
        return true;
    }
    if !hooks().contains(&method) {
        return false;
    }
    let Some(receiver) = call_receiver(node) else {
        return false;
    };
    let mut ancestor = node.parent();
    while let Some(parent) = ancestor {
        if bare_rspec_call(source, parent) == Some(b"configure") {
            if let Some(parameters) =
                call_block(parent).and_then(|b| b.child_by_field_name("parameters"))
            {
                let mut cursor = parameters.walk();
                return parameters.named_children(&mut cursor).any(|parameter| {
                    node_bytes(source, parameter) == node_bytes(source, receiver)
                });
            }
        }
        ancestor = parent.parent();
    }
    false
}

pub(super) fn call_args(node: Node<'_>) -> Vec<Node<'_>> {
    crate::cop::shared::argument_nodes(node)
        .into_iter()
        .filter(|child| child.kind() != "comment")
        .collect()
}
pub(super) fn code_children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter(|child| child.kind() != "comment")
        .collect()
}
pub(super) fn preserves_comments(
    source: &SourceFile,
    node: Node<'_>,
    start: usize,
    end: usize,
    replacement: &str,
) -> bool {
    let mut pending = vec![node];
    while let Some(current) = pending.pop() {
        if current.kind() == "comment"
            && current.start_byte() >= start
            && current.end_byte() <= end
            && !replacement
                .as_bytes()
                .windows(current.end_byte() - current.start_byte())
                .any(|part| part == node_bytes(source, current))
        {
            return false;
        }
        let mut cursor = current.walk();
        pending.extend(current.named_children(&mut cursor));
    }
    true
}
pub(super) fn preserving_replace(
    source: &SourceFile,
    node: Node<'_>,
    corrections: &mut Option<&mut Vec<crate::correction::Correction>>,
    start: usize,
    end: usize,
    replacement: impl Into<String>,
    cop: &'static str,
) -> bool {
    let replacement = replacement.into();
    if !preserves_comments(source, node, start, end, &replacement) {
        return false;
    }
    crate::cop::shared::push_replace(corrections, start, end, replacement, cop)
}
