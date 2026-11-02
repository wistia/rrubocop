use crate::cop::shared::ruby_arguments::arguments;
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, collect_comments, is_setter_call, named_kids,
    node_bytes, push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

fn signature(node: Node<'_>) -> Option<Node<'_>> {
    let parameters = node.child_by_field_name("parameters")?;
    let mut cursor = parameters.walk();
    parameters
        .named_children(&mut cursor)
        .filter(|node| node.kind() != "comment")
        .last()
        .filter(|node| {
            node.kind() == "block_parameter" && node.child_by_field_name("name").is_some()
        })
}

fn walk(node: Node<'_>) -> Vec<Node<'_>> {
    let mut nodes = Vec::new();
    let mut pending = vec![node];
    while let Some(node) = pending.pop() {
        nodes.push(node);
        pending.extend(named_kids(node).into_iter().rev());
    }
    nodes
}
fn assignment(source: &SourceFile, node: Node<'_>, name: &[u8]) -> bool {
    let target = match node.kind() {
        "assignment" | "operator_assignment" => node.child_by_field_name("left"),
        "for" => node.child_by_field_name("pattern"),
        "exception_variable" => Some(node),
        _ => None,
    };
    fn assigned_name(source: &SourceFile, target: Node<'_>, name: &[u8]) -> bool {
        if matches!(
            target.kind(),
            "call" | "element_reference" | "scope_resolution"
        ) {
            return false;
        }
        target.kind() == "identifier"
            && node_bytes(source, target) == name
            && crate::cop::ported::local_identifier(source, target)
            || named_kids(target)
                .iter()
                .any(|node| assigned_name(source, *node, name))
    }
    target.is_some_and(|target| assigned_name(source, target, name))
}
fn proc_call(source: &SourceFile, node: Node<'_>, name: &[u8]) -> bool {
    if node.kind() != "call"
        || is_setter_call(node)
        || node
            .child_by_field_name("operator")
            .is_some_and(|op| node_bytes(source, op) == b"&.")
    {
        return false;
    }
    let method = call_method_name(source, node);
    let is_call = method == Some(b"call")
        || method.is_none() && node.child_by_field_name("arguments").is_some();
    is_call
        && call_receiver(node).is_some_and(|n| {
            n.kind() == "identifier"
                && node_bytes(source, n) == name
                && crate::cop::ported::local_identifier(source, n)
        })
}
fn sole_block_shadows(source: &SourceFile, body: Node<'_>, name: &[u8]) -> bool {
    let children = named_kids(body)
        .into_iter()
        .filter(|n| n.kind() != "comment")
        .collect::<Vec<_>>();
    if children.len() != 1 {
        return false;
    }
    children[0]
        .child_by_field_name("block")
        .and_then(|block| block.child_by_field_name("parameters"))
        .is_some_and(|parameters| {
            named_kids(parameters)
                .iter()
                .any(|parameter| node_bytes(source, *parameter) == name)
        })
}
fn present(source: &SourceFile, condition: Node<'_>, binding: usize, positive: bool) -> bool {
    if condition.kind() == "identifier"
        && crate::cop::ported::local_binding(source, condition) == Some(binding)
    {
        return positive;
    }
    if condition.kind() == "call"
        && call_receiver(condition).is_none()
        && call_method_name(source, condition) == Some(b"block_given?")
        && argument_nodes(condition).is_empty()
        && condition.child_by_field_name("block").is_none()
    {
        return positive;
    }
    if condition.kind() == "identifier"
        && node_bytes(source, condition) == b"block_given?"
        && crate::cop::ported::bare_call_identifier(source, condition)
    {
        return positive;
    }
    if condition.kind() == "unary"
        && condition
            .child_by_field_name("operator")
            .is_some_and(|n| matches!(node_bytes(source, n), b"!" | b"not"))
    {
        return condition
            .child_by_field_name("operand")
            .is_some_and(|operand| present(source, operand, binding, !positive));
    }
    if condition.kind() == "parenthesized_statements" {
        let values = named_kids(condition);
        return values.len() == 1 && present(source, values[0], binding, positive);
    }
    if condition.kind() == "binary"
        && condition.child_by_field_name("operator").is_some_and(|n| {
            if positive {
                matches!(node_bytes(source, n), b"&&" | b"and")
            } else {
                matches!(node_bytes(source, n), b"||" | b"or")
            }
        })
    {
        return ["left", "right"]
            .iter()
            .filter_map(|field| condition.child_by_field_name(field))
            .any(|side| present(source, side, binding, positive));
    }
    false
}
fn block_present(source: &SourceFile, call: Node<'_>, method: Node<'_>, binding: usize) -> bool {
    let mut child = call;
    let mut parent = call.parent();
    while let Some(node) = parent {
        if node == method {
            break;
        }
        if matches!(
            node.kind(),
            "method" | "singleton_method" | "class" | "module"
        ) {
            return false;
        }
        if matches!(
            node.kind(),
            "if" | "unless" | "if_modifier" | "unless_modifier" | "conditional"
        ) {
            if let Some(condition) = node.child_by_field_name("condition") {
                let in_true = node.child_by_field_name("consequence") == Some(child)
                    || node.child_by_field_name("body") == Some(child);
                let in_false = node.child_by_field_name("alternative") == Some(child);
                if (in_true || in_false)
                    && present(
                        source,
                        condition,
                        binding,
                        in_true != matches!(node.kind(), "unless" | "unless_modifier"),
                    )
                {
                    return true;
                }
            }
        }
        if node.kind() == "binary" && node.child_by_field_name("right") == Some(child) {
            if let (Some(left), Some(operator)) = (
                node.child_by_field_name("left"),
                node.child_by_field_name("operator"),
            ) {
                let op = node_bytes(source, operator);
                if matches!(op, b"&&" | b"and" | b"||" | b"or")
                    && present(source, left, binding, matches!(op, b"&&" | b"and"))
                {
                    return true;
                }
            }
        }
        child = node;
        parent = node.parent();
    }
    false
}
fn call_override_visible(source: &SourceFile, mut node: Node<'_>) -> bool {
    while let Some(parent) = node.parent() {
        node = parent;
    }
    struct VisibleCallOverrides(bool);
    source
        .cached(0, || {
            VisibleCallOverrides(walk(node).iter().any(|node| {
                matches!(node.kind(), "method" | "singleton_method")
                    && node.child_by_field_name("name").is_some_and(|name| {
                        matches!(node_bytes(source, name), b"call" | b"block_given?")
                    })
                    || matches!(
                        call_method_name(source, *node),
                        Some(b"define_method" | b"define_singleton_method" | b"alias_method")
                    ) && argument_nodes(*node)
                        .first()
                        .and_then(|name| crate::cop::ported::static_symbol(source, *name))
                        .as_deref()
                        == Some("call")
            }))
        })
        .0
}

fn safe_binding(
    source: &SourceFile,
    call: Node<'_>,
    method: Node<'_>,
    parameter: Node<'_>,
    nodes: &[Node<'_>],
) -> bool {
    let Some(binding) = crate::cop::ported::local_binding(source, parameter) else {
        return false;
    };
    if call_override_visible(source, call) {
        return false;
    }
    if call_receiver(call).and_then(|receiver| crate::cop::ported::local_binding(source, receiver))
        != Some(binding)
        || nodes
            .iter()
            .any(|n| matches!(n.kind(), "match_pattern" | "test_pattern" | "case_match"))
    {
        return false;
    }
    block_present(source, call, method, binding)
}

fn valid_replacement(source: &SourceFile, call: Node<'_>, end: usize, text: &str) -> bool {
    let original = &source.as_bytes()[call.start_byte()..end];
    if std::str::from_utf8(original).is_err()
        || original.contains(&b'\n')
            && walk(call)
                .iter()
                .any(|node| node.kind() == "identifier" && node_bytes(source, *node) == b"__LINE__")
    {
        return false;
    }
    let mut candidate = source.as_bytes().to_vec();
    candidate.splice(call.start_byte()..end, text.bytes());
    let source = SourceFile::from_bytes("candidate.rb", candidate);
    crate::parse::parse_ruby(&source).is_ok_and(|tree| !tree.root_node().has_error())
}

pub struct RedundantBlockCall;
impl Cop for RedundantBlockCall {
    fn name(&self) -> &'static str {
        "Performance/RedundantBlockCall"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["method", "singleton_method"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let Some(parameter) = signature(node) else {
            return;
        };
        let name = node_bytes(source, parameter.child_by_field_name("name").unwrap());
        let Some(body) = node.child_by_field_name("body") else {
            return;
        };
        let nodes = walk(body);
        if nodes.iter().any(|n| assignment(source, *n, name))
            || sole_block_shadows(source, body, name)
        {
            return;
        }
        let calls = nodes
            .iter()
            .copied()
            .filter(|n| proc_call(source, *n, name))
            .collect::<Vec<_>>();
        if calls.iter().any(|call| {
            argument_nodes(*call)
                .iter()
                .any(|arg| arg.kind() == "block_argument")
        }) {
            return;
        }
        for call in calls {
            if call.child_by_field_name("block").is_some() {
                continue;
            }
            let end = super::send_range::end(call);
            let (line, col) = source.offset_to_line_col(call.start_byte());
            let message = format!(
                "Use `yield` instead of `{}.call`.",
                String::from_utf8_lossy(name)
            );
            let index = diagnostics
                .iter()
                .position(|d| {
                    d.cop_name == self.name()
                        && d.location.line == line
                        && d.location.column == col
                        && d.message == message
                })
                .unwrap_or_else(|| {
                    let mut d = self.diagnostic(source, line, col, message);
                    d.correctable = !config.autocorrect_disabled();
                    d.highlight_length =
                        String::from_utf8_lossy(&source.as_bytes()[call.start_byte()..end])
                            .split('\n')
                            .next()
                            .unwrap_or("")
                            .chars()
                            .count();
                    diagnostics.push(d);
                    diagnostics.len() - 1
                });
            if corrections.is_none()
                || config.autocorrect_disabled()
                || !safe_binding(source, call, node, parameter, &nodes)
                || collect_comments(call)
                    .iter()
                    .any(|comment| comment.start_byte() < end)
            {
                continue;
            }
            let args = arguments(call);
            let parens = call
                .child_by_field_name("arguments")
                .is_some_and(|args| node_bytes(source, args).starts_with(b"("));
            let mut text = "yield".to_owned();
            if !args.is_empty() {
                text.push(if parens { '(' } else { ' ' });
                text.push_str(
                    &args
                        .iter()
                        .map(|arg| {
                            String::from_utf8_lossy(&source.as_bytes()[arg.start()..arg.end()])
                        })
                        .collect::<Vec<_>>()
                        .join(", "),
                );
                if parens {
                    text.push(')');
                }
            }
            if !valid_replacement(source, call, end, &text) {
                continue;
            }
            if push_replace(&mut corrections, call.start_byte(), end, text, self.name()) {
                diagnostics[index].corrected = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn corrected(code: &str, config: CopConfig) -> (String, Vec<Diagnostic>) {
        let source = SourceFile::from_bytes("example.rb", code.as_bytes().to_vec());
        let tree = crate::parse::parse_ruby(&source).unwrap();
        let mut diagnostics = Vec::new();
        let mut edits = Vec::new();
        crate::cop::walker::BatchedWalker::new(vec![&RedundantBlockCall], vec![&config]).walk(
            &source,
            tree.root_node(),
            &mut diagnostics,
            Some(&mut edits),
        );
        (
            String::from_utf8(
                crate::correction::CorrectionSet::from_vec(edits).apply(source.as_bytes()),
            )
            .unwrap(),
            diagnostics,
        )
    }
    #[test]
    fn guarded_block_calls_preserve_bindings_and_missing_block_errors() {
        let optional = "def example(&block); block.call; end";
        let (after, diagnostics) = corrected(optional, CopConfig::default());
        assert_eq!(after, optional);
        assert_eq!(diagnostics.len(), 1);
        let shadowed =
            "def example(&block); if block; marker; items.each { |block| block.call }; end; end";
        let (after, diagnostics) = corrected(shadowed, CopConfig::default());
        assert_eq!(after, shadowed);
        assert_eq!(diagnostics.len(), 1);
        let safe =
            "def example(&block); if block; items.each { |item| block.call(item) }; end; end";
        assert_eq!(
            corrected(safe, CopConfig::default()).0,
            "def example(&block); if block; items.each { |item| yield(item) }; end; end"
        );
    }
}
