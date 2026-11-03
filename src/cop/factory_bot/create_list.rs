//! Prefer one factory-list call or the configured repeated-create style.
use crate::cop::shared::{
    argument_nodes, call_method_name, call_receiver, collect_comments, for_each_descendant,
    method_node, named_kids, node_bytes, node_text, push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;
pub struct CreateList;
impl Cop for CreateList {
    fn name(&self) -> &'static str {
        "FactoryBot/CreateList"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn safe_autocorrect(&self) -> bool {
        false
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "array"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"times", b"map", b"new", b"create_list"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let n_times = config.get_str("EnforcedStyle", "create_list") == "n_times";
        if node.kind() == "array" {
            let values: Vec<_> = named_kids(node)
                .into_iter()
                .filter(|n| n.kind() != "comment")
                .collect();
            if values.len() < 2
                || !values
                    .iter()
                    .all(|n| factory_call(source, *n, b"create", config))
                || values
                    .iter()
                    .any(|n| node_bytes(source, *n) != node_bytes(source, values[0]))
            {
                return;
            }
            let message = if n_times || dynamic_arguments(source, values[0], config) {
                format!("Prefer {}.times.map.", values.len())
            } else {
                "Prefer create_list.".into()
            };
            let replacement = std::str::from_utf8(node_bytes(source, values[0]))
                .ok()
                .map(|value| format!("{}.times.map {{ {value} }}", values.len()));
            report(
                self,
                source,
                node,
                node.end_byte(),
                message,
                replacement.map(|r| (node, r)),
                config,
                diagnostics,
                &mut corrections,
            );
            return;
        }
        if n_times {
            if !factory_call(source, node, b"create_list", config) {
                return;
            }
            let args = argument_nodes(node);
            if args.len() < 2 || integer(source, args[1]).is_none_or(|n| n < 2) {
                return;
            }
            let Some(method) = method_node(node) else {
                return;
            };
            let replacement = render_times(source, node, &args);
            report(
                self,
                source,
                method,
                method.end_byte(),
                format!("Prefer {}.times.map.", integer(source, args[1]).unwrap()),
                replacement.map(|r| (node, r)),
                config,
                diagnostics,
                &mut corrections,
            );
            return;
        }
        let Some(count) = repeat_count(source, node) else {
            return;
        };
        if integer(source, count).is_none_or(|n| n < 2) {
            return;
        }
        let Some(block) = node.child_by_field_name("block") else {
            return;
        };
        if config.get_f64("TargetRubyVersion", 3.4) >= 2.7
            && crate::cop::shared::ruby_expression::numbered_bindings(source, block).is_some()
        {
            return;
        }
        let Some(body) = block.child_by_field_name("body") else {
            return;
        };
        let statements: Vec<_> = named_kids(body)
            .into_iter()
            .filter(|n| n.kind() != "comment")
            .collect();
        if statements.len() != 1 {
            return;
        }
        let create = statements[0];
        if !factory_call(source, create, b"create", config)
            || dynamic_arguments(source, create, config)
            || used_parameter(source, block, body)
        {
            return;
        }
        let end = node_bytes(source, node)[..block.start_byte() - node.start_byte()]
            .iter()
            .rposition(|b| !b.is_ascii_whitespace())
            .map_or(block.start_byte(), |last| node.start_byte() + last + 1);
        let replacement = if safe_navigation_argument(source, create) {
            None
        } else {
            render_list(source, create, count)
        };
        report(
            self,
            source,
            node,
            end,
            "Prefer create_list.".into(),
            replacement.map(|r| (node, r)),
            config,
            diagnostics,
            &mut corrections,
        );
    }
}

fn report<'a>(
    cop: &CreateList,
    source: &SourceFile,
    node: Node<'a>,
    end: usize,
    message: String,
    replacement: Option<(Node<'a>, String)>,
    config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (line, col) = source.offset_to_line_col(node.start_byte());
    let bytes = &source.as_bytes()[node.start_byte()..end];
    let line_end = bytes
        .iter()
        .position(|b| *b == b'\n')
        .unwrap_or(bytes.len());
    let mut d = cop.diagnostic(source, line, col, message);
    d.highlight_length = String::from_utf8_lossy(&bytes[..line_end]).chars().count();
    d.correctable = replacement.is_some() && !config.autocorrect_disabled();
    if let Some((target, replacement)) = replacement {
        if collect_comments(target).is_empty() {
            d.corrected = push_replace(
                corrections,
                target.start_byte(),
                target.end_byte(),
                replacement,
                cop.name(),
            );
        } else {
            d.correctable = false;
        }
    }
    diagnostics.push(d);
}
fn factory_call(source: &SourceFile, node: Node<'_>, method: &[u8], config: &CopConfig) -> bool {
    if call_method_name(source, node) != Some(method)
        || node
            .child_by_field_name("operator")
            .is_some_and(|op| node_bytes(source, op) == b"&.")
    {
        return false;
    }
    let receiver = call_receiver(node);
    if !receiver.is_some_and(|n| {
        matches!(
            node_text(source, n).as_str(),
            "FactoryBot" | "FactoryGirl" | "::FactoryBot" | "::FactoryGirl"
        )
    }) && (receiver.is_some() || config.get_bool("ExplicitOnly", false))
    {
        return false;
    }
    argument_nodes(node)
        .first()
        .is_some_and(|arg| matches!(arg.kind(), "simple_symbol" | "delimited_symbol"))
}
fn integer(source: &SourceFile, node: Node<'_>) -> Option<u64> {
    if node.kind() != "integer" {
        return None;
    }
    let value = node_text(source, node).replace('_', "");
    for (prefix, radix) in [
        ("0x", 16),
        ("0X", 16),
        ("0b", 2),
        ("0B", 2),
        ("0o", 8),
        ("0O", 8),
        ("0d", 10),
        ("0D", 10),
    ] {
        if let Some(value) = value.strip_prefix(prefix) {
            return u64::from_str_radix(value, radix).ok();
        }
    }
    value.parse().ok()
}
fn repeat_count<'a>(source: &SourceFile, node: Node<'a>) -> Option<Node<'a>> {
    let receiver = call_receiver(node)?;
    match call_method_name(source, node)? {
        b"times" if argument_nodes(node).is_empty() => Some(receiver),
        b"map"
            if call_method_name(source, receiver) == Some(b"times")
                && argument_nodes(node).is_empty()
                && argument_nodes(receiver).is_empty() =>
        {
            call_receiver(receiver)
        }
        b"new"
            if matches!(node_text(source, receiver).as_str(), "Array" | "::Array")
                && argument_nodes(node).len() == 1 =>
        {
            argument_nodes(node).first().copied()
        }
        _ => None,
    }
}
fn dynamic_arguments(source: &SourceFile, node: Node<'_>, config: &CopConfig) -> bool {
    let mut found = false;
    for argument in argument_nodes(node).iter().skip(1) {
        for_each_descendant(*argument, |n| {
            found |= crate::cop::shared::ruby_expression::regular_send_name(source, n).is_some()
                || (config.get_f64("TargetRubyVersion", 3.4) < 2.7
                    && legacy_numbered_read(source, n))
        });
    }
    found
}

fn legacy_numbered_read(source: &SourceFile, node: Node<'_>) -> bool {
    let Some(binding) = crate::cop::ported::local_binding(source, node) else {
        return false;
    };
    let mut ancestor = node.parent();
    while let Some(parent) = ancestor {
        if crate::cop::shared::ruby_expression::numbered_bindings(source, parent)
            .is_some_and(|bindings| bindings.contains(&binding))
        {
            return true;
        }
        ancestor = parent.parent();
    }
    false
}

fn safe_navigation_argument(source: &SourceFile, node: Node<'_>) -> bool {
    let mut found = false;
    for argument in argument_nodes(node).iter().skip(1) {
        for_each_descendant(*argument, |node| {
            found |= node.kind() == "call"
                && node
                    .child_by_field_name("operator")
                    .is_some_and(|op| node_bytes(source, op) == b"&.");
        });
    }
    found
}

fn used_parameter(source: &SourceFile, block: Node<'_>, body: Node<'_>) -> bool {
    let Some(parameters) = block.child_by_field_name("parameters") else {
        return false;
    };
    let names: Vec<_> = named_kids(parameters)
        .into_iter()
        .filter(|n| n.kind() == "identifier")
        .map(|n| node_bytes(source, n))
        .collect();
    let mut used = false;
    for_each_descendant(body, |n| {
        used |= n.kind() == "identifier" && names.contains(&node_bytes(source, n))
    });
    used
}
fn invocation(source: &SourceFile, node: Node<'_>, method: &str, arguments: String) -> String {
    let receiver = call_receiver(node)
        .map(|n| format!("{}.", node_text(source, n)))
        .unwrap_or_default();
    let parens = node.child_by_field_name("block").is_some()
        || node
            .child_by_field_name("arguments")
            .is_some_and(|args| node_bytes(source, args).starts_with(b"("));
    if parens {
        format!("{receiver}{method}({arguments})")
    } else {
        format!("{receiver}{method} {arguments}")
    }
}
fn factory_block(source: &SourceFile, node: Node<'_>) -> Option<String> {
    let block = node.child_by_field_name("block")?;
    std::str::from_utf8(node_bytes(source, block))
        .ok()
        .map(str::to_owned)
}
fn render_times(source: &SourceFile, node: Node<'_>, args: &[Node<'_>]) -> Option<String> {
    std::str::from_utf8(node_bytes(source, node)).ok()?;
    let mut arguments = vec![node_text(source, args[0])];
    arguments.extend(args[2..].iter().map(|n| node_text(source, *n)));
    let mut create = invocation(source, node, "create", arguments.join(", "));
    if node.child_by_field_name("block").is_some() {
        create.push(' ');
        create.push_str(&factory_block(source, node)?);
    }
    Some(format!(
        "{}.times.map {{ {create} }}",
        node_text(source, args[1])
    ))
}
fn render_list(source: &SourceFile, node: Node<'_>, count: Node<'_>) -> Option<String> {
    std::str::from_utf8(node_bytes(source, node)).ok()?;
    let args = argument_nodes(node);
    let mut arguments = vec![node_text(source, args[0]), node_text(source, count)];
    arguments.extend(args[1..].iter().map(|n| node_text(source, *n)));
    let mut replacement = invocation(source, node, "create_list", arguments.join(", "));
    if let Some(block) = node.child_by_field_name("block") {
        let body = block.child_by_field_name("body")?;
        let statements: Vec<_> = named_kids(body)
            .into_iter()
            .filter(|n| n.kind() != "comment")
            .collect();
        let first = statements.first()?;
        let last = statements.last()?;
        let text =
            std::str::from_utf8(&source.as_bytes()[first.start_byte()..last.end_byte()]).ok()?;
        let params = block
            .child_by_field_name("parameters")
            .map(|n| node_text(source, n))
            .unwrap_or_default();
        if statements.len() > 1 {
            let indent = source.offset_to_line_col(first.start_byte()).1;
            let mut parent = node.parent();
            let mut end_indent = 0;
            while let Some(ancestor) = parent {
                if ancestor.kind() == "call" && ancestor.child_by_field_name("block").is_some() {
                    end_indent = source.offset_to_line_col(ancestor.start_byte()).1;
                    break;
                }
                parent = ancestor.parent();
            }
            replacement.push_str(&format!(
                " do {params}\n{}{text}\n{}end",
                " ".repeat(indent),
                " ".repeat(end_indent)
            ));
        } else {
            replacement.push_str(&format!(" {{ {params} {text} }}"));
        }
    }
    Some(replacement)
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(CreateList, "cops/factory_bot/create_list");

    #[test]
    fn refuses_source_losing_comment_and_raw_byte_rewrites() {
        for before in [
            b"3.times { # keep this\n create :user }\n".as_slice(),
            b"3.times { create :user, name: \"\xff\" }\n",
            b"[create(:user), # keep this\n create(:user)]\n",
        ] {
            let source = SourceFile::from_bytes("spec/example_spec.rb", before.to_vec());
            let tree = crate::parse::parse_ruby(&source).unwrap();
            let config = CopConfig::default();
            let mut diagnostics = vec![];
            let mut edits = vec![];
            crate::cop::walker::BatchedWalker::new(vec![&CreateList], vec![&config]).walk(
                &source,
                tree.root_node(),
                &mut diagnostics,
                Some(&mut edits),
            );
            assert!(edits.is_empty());
            assert_eq!(diagnostics.len(), 1);
            assert!(!diagnostics[0].correctable);
        }
    }
}
