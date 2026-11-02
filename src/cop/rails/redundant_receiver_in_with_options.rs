//! Remove option-merger receivers only when the rewritten block preserves its bindings.
use crate::cop::shared::ruby_expression::{ShorthandValue, regular_send_receiver, shorthand_value};
use crate::cop::shared::{
    call_method_name, collect_comments, command_unary_plus_call, for_each_descendant,
    is_setter_call, method_node, named_kids, node_bytes, node_text, push_replace,
};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use std::collections::{BTreeMap, HashSet};
use tree_sitter::Node;
pub struct RedundantReceiverInWithOptions;
struct Send<'a> {
    node: Node<'a>,
    receiver: Option<Node<'a>>,
    selector: Option<Node<'a>>,
    dot: Option<Node<'a>>,
}
fn regular_sends<'a>(source: &SourceFile, root: Node<'a>) -> Vec<Send<'a>> {
    let mut found = BTreeMap::new();
    for_each_descendant(root, |node| {
        if node.kind() == "binary" {
            if let Some(call) = command_unary_plus_call(source, node) {
                let dot = call
                    .selector
                    .parent()
                    .filter(|n| n.kind() == "call")
                    .and_then(|n| n.child_by_field_name("operator"));
                if dot.is_some_and(|n| node_bytes(source, n) == b"&.") {
                    return;
                }
                found.insert(
                    call.selector.start_byte(),
                    Send {
                        node,
                        receiver: call.receiver,
                        selector: Some(call.selector),
                        dot,
                    },
                );
                return;
            }
        }
        if node.kind() == "lambda" {
            found.insert(
                node.start_byte(),
                Send {
                    node,
                    receiver: None,
                    selector: None,
                    dot: None,
                },
            );
            return;
        }
        if !matches!(
            node.kind(),
            "call" | "element_reference" | "binary" | "unary" | "identifier" | "hash_key_symbol"
        ) {
            return;
        }
        let Some(receiver) = regular_send_receiver(source, node) else {
            return;
        };
        let selector = method_node(node)
            .or_else(|| node.child_by_field_name("operator"))
            .or_else(|| (node.kind() == "identifier").then_some(node));
        let dot = if node.kind() == "call" {
            node.child_by_field_name("operator")
        } else {
            None
        };
        found.insert(
            selector.map_or(node.start_byte(), |n| n.start_byte()),
            Send {
                node,
                receiver,
                selector,
                dot,
            },
        );
    });
    found.into_values().collect()
}
fn parameter_name(source: &SourceFile, node: Node<'_>) -> Option<String> {
    if node.kind() == "destructured_parameter" {
        return None;
    }
    if node.kind() == "identifier" {
        Some(node_text(source, node))
    } else {
        node.child_by_field_name("name")
            .or_else(|| {
                named_kids(node)
                    .into_iter()
                    .find(|n| n.kind() == "identifier")
            })
            .map(|n| node_text(source, n))
    }
}
impl Cop for RedundantReceiverInWithOptions {
    fn name(&self) -> &'static str {
        "Rails/RedundantReceiverInWithOptions"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn interested_call_names(&self) -> &'static [&'static [u8]] {
        &[b"with_options"]
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if call_method_name(source, node) != Some(b"with_options") || is_setter_call(node) {
            return;
        }
        let Some(block) = node.child_by_field_name("block") else {
            return;
        };
        let Some(body) = block.child_by_field_name("body") else {
            return;
        };
        let modern = config.get_f64("TargetRubyVersion", 3.4) >= 2.7;
        let numbers = modern
            .then(|| crate::cop::ported::numbered_bindings(source, block))
            .flatten();
        let params = block.child_by_field_name("parameters");
        let args: Vec<_> = params.map_or_else(Vec::new, |n| {
            named_kids(n)
                .into_iter()
                .filter(|n| n.kind() != "comment")
                .collect()
        });
        let name = if numbers.is_some() {
            "_1".into()
        } else {
            let Some(first) = args.first().copied() else {
                return;
            };
            let Some(name) = parameter_name(source, first) else {
                return;
            };
            name
        };
        let mut nested = false;
        for_each_descendant(body, |n| {
            if matches!(n.kind(), "block" | "do_block" | "lambda")
                && !(modern && crate::cop::ported::numbered_bindings(source, n).is_some())
            {
                nested = true;
            }
        });
        if nested {
            return;
        }
        let sends = regular_sends(source, body);
        if sends.is_empty()
            || sends.iter().any(|send| {
                !send.receiver.is_some_and(|receiver| {
                    receiver.kind() == "identifier"
                        && node_text(source, receiver) == name
                        && crate::cop::ported::local_identifier(source, receiver)
                })
            })
        {
            return;
        }
        let receivers: HashSet<_> = sends
            .iter()
            .filter_map(|send| send.receiver)
            .map(|n| n.start_byte())
            .collect();
        let bindings: Vec<_> = numbers.clone().unwrap_or_else(|| {
            args.iter()
                .filter_map(|arg| crate::cop::ported::local_binding(source, *arg))
                .collect()
        });
        let mut guarded = (!numbers.is_some() && args.iter().any(|arg| arg.kind() != "identifier"))
            || bindings.is_empty();
        for_each_descendant(body, |n| {
            guarded |= matches!(
                n.kind(),
                "self"
                    | "instance_variable"
                    | "super"
                    | "method"
                    | "singleton_method"
                    | "class"
                    | "module"
                    | "singleton_class"
            );
            if matches!(n.kind(), "identifier" | "hash_key_symbol")
                && crate::cop::ported::local_binding(source, n)
                    .is_some_and(|id| bindings.contains(&id))
                && !receivers.contains(&n.start_byte())
            {
                guarded = true;
            }
            if shorthand_value(source, n) == Some(ShorthandValue::Constant)
                && !node_text(source, n)
                    .chars()
                    .next()
                    .is_some_and(char::is_uppercase)
            {
                guarded = true;
            }
            if n.kind() == "pair"
                && n.child_by_field_name("value").is_none()
                && n.child_by_field_name("key")
                    .is_some_and(|key| node_text(source, key) == name)
            {
                guarded = true;
            }
        });
        let mut edits = Vec::new();
        for send in &sends {
            let receiver = send.receiver.unwrap();
            let (line, _) = source.offset_to_line_col(receiver.start_byte());
            guarded |= send.selector.is_some_and(|selector| {
                include_str!("../../resources/option_merger_private_methods_ruby_3_4.txt")
                    .lines()
                    .any(|method| method.as_bytes() == node_bytes(source, selector))
            });
            guarded |= crate::parse::directives::cached(source).suppresses(self.name(), line)
                || is_setter_call(send.node);
            edits.push((receiver.start_byte(), receiver.end_byte()));
            if let Some(dot) = send.dot {
                edits.push((dot.start_byte(), dot.end_byte()));
            } else {
                guarded = true;
            }
        }
        if numbers.is_none() {
            if let Some(params) = params {
                let mut start = params.start_byte();
                while start > 0 && source.as_bytes()[start - 1] == b' ' {
                    start -= 1;
                }
                edits.push((start, params.end_byte()));
            }
        }
        edits.sort_unstable();
        edits.dedup();
        guarded |= collect_comments(block).iter().any(|n| {
            edits
                .iter()
                .any(|(start, end)| *start < n.end_byte() && *end > n.start_byte())
        });
        let loses_lines = edits
            .iter()
            .any(|(start, end)| source.as_bytes()[*start..*end].contains(&b'\n'));
        if loses_lines {
            for_each_descendant(body, |n| {
                guarded |= n.kind() == "identifier"
                    && crate::cop::ported::bare_call_identifier(source, n)
                    && node_bytes(source, n) == b"__LINE__"
            });
        }
        let mut proposed = source.as_bytes().to_vec();
        for (start, end) in edits.iter().rev() {
            proposed.drain(*start..*end);
        }
        let proposed = SourceFile::from_bytes("with_options.rb", proposed);
        if !guarded {
            if let Ok(tree) = crate::parse::parse_ruby(&proposed) {
                if tree.root_node().has_error() {
                    guarded = true;
                } else {
                    let new_sends = regular_sends(&proposed, tree.root_node());
                    for send in &sends {
                        let Some(selector) = send.selector else {
                            guarded = true;
                            continue;
                        };
                        let removed: usize = edits
                            .iter()
                            .filter(|(_, end)| *end <= selector.start_byte())
                            .map(|(start, end)| end - start)
                            .sum();
                        let byte = selector.start_byte() - removed;
                        guarded |= !new_sends.iter().any(|new| {
                            new.receiver.is_none()
                                && new.selector.is_some_and(|n| {
                                    n.start_byte() == byte
                                        && node_bytes(&proposed, n) == node_bytes(source, selector)
                                })
                        });
                    }
                }
            } else {
                guarded = true;
            }
        }
        let correctable = !guarded && !config.autocorrect_disabled();
        let mut corrected = false;
        if correctable {
            for (start, end) in edits {
                corrected |= push_replace(&mut corrections, start, end, "", self.name());
            }
        }
        for send in sends {
            let receiver = send.receiver.unwrap();
            let (line, col) = source.offset_to_line_col(receiver.start_byte());
            let mut d = self.diagnostic(
                source,
                line,
                col,
                "Redundant receiver in `with_options`.".into(),
            );
            d.highlight_length = node_text(source, receiver).chars().count();
            d.correctable = correctable;
            d.corrected = corrected;
            diagnostics.push(d);
        }
    }
}
