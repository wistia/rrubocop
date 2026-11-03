use super::{HashSyntax, operator};
use crate::cop::shared::{collect_comments, named_kids, node_bytes, push_replace};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use std::sync::Arc;
use tree_sitter::Node;

fn dispatch(pair: Node<'_>) -> Option<Node<'_>> {
    let hash = pair.parent()?;
    let parent = hash.parent()?;
    matches!(parent.kind(), "call" | "super" | "yield").then_some(parent)
}
fn parenthesized(source: &SourceFile, call: Node<'_>) -> bool {
    call.child_by_field_name("arguments")
        .or_else(|| {
            named_kids(call)
                .into_iter()
                .find(|n| n.kind() == "argument_list")
        })
        .is_some_and(|args| node_bytes(source, args).starts_with(b"("))
}
fn modifier(node: Node<'_>) -> bool {
    let mut ancestor = node.parent();
    while let Some(parent) = ancestor {
        if matches!(
            parent.kind(),
            "if_modifier"
                | "unless_modifier"
                | "while_modifier"
                | "until_modifier"
                | "rescue_modifier"
        ) {
            return true;
        }
        ancestor = parent.parent();
    }
    false
}
fn omittable(source: &SourceFile, pair: Node<'_>) -> bool {
    let (Some(key), Some(value)) = (
        pair.child_by_field_name("key"),
        pair.child_by_field_name("value"),
    ) else {
        return false;
    };
    if !matches!(key.kind(), "hash_key_symbol" | "simple_symbol")
        || !matches!(value.kind(), "identifier" | "call")
        || matches!(
            node_bytes(source, value),
            b"__FILE__" | b"__LINE__" | b"__ENCODING__"
        )
        || node_bytes(source, key) != node_bytes(source, value)
        || node_bytes(source, key)
            .last()
            .is_some_and(|b| matches!(b, b'!' | b'?'))
    {
        return false;
    }
    if let Some(call) = dispatch(pair) {
        let braced = pair.parent().is_some_and(|p| p.kind() == "hash");
        let receiver = call.child_by_field_name("receiver") == pair.parent();
        if !braced && !receiver && !parenthesized(source, call) && modifier(call) {
            return false;
        }
    }
    true
}
fn next_expression(node: Node<'_>) -> bool {
    let mut sibling = node.next_named_sibling();
    while let Some(next) = sibling {
        if !matches!(next.kind(), "comment" | "empty_statement") {
            return true;
        }
        sibling = next.next_named_sibling();
    }
    false
}
fn last_expression(node: Node<'_>) -> bool {
    if next_expression(node) {
        return false;
    }
    let mut ancestor = node.parent();
    while let Some(parent) = ancestor {
        if matches!(parent.kind(), "assignment" | "operator_assignment") {
            return if parent
                .parent()
                .is_some_and(|p| matches!(p.kind(), "assignment" | "operator_assignment"))
            {
                last_expression(parent.parent().unwrap())
            } else {
                !next_expression(parent)
            };
        }
        ancestor = parent.parent();
    }
    true
}
struct Siblings {
    last_index: u32,
    has_omitted: bool,
    has_needed: bool,
}
fn siblings(source: &SourceFile, pair: Node<'_>) -> Arc<Siblings> {
    let parent = pair.parent().unwrap();
    source.cached(parent.id(), || {
        let mut result = Siblings {
            last_index: 0,
            has_omitted: false,
            has_needed: false,
        };
        for index in 0..parent.named_child_count() {
            let child = parent.named_child(index as u32).unwrap();
            if child.kind() != "pair" {
                continue;
            }
            result.last_index = index as u32;
            result.has_omitted |= child.child_by_field_name("value").is_none();
            result.has_needed |=
                child.child_by_field_name("value").is_some() && !omittable(source, child);
        }
        result
    })
}
fn last_pair<'a>(source: &SourceFile, pair: Node<'a>) -> Node<'a> {
    pair.parent()
        .unwrap()
        .named_child(siblings(source, pair).last_index)
        .unwrap()
}
fn wrap_call<'a>(source: &SourceFile, pair: Node<'a>) -> Option<Node<'a>> {
    let last = last_pair(source, pair);
    let key = last.child_by_field_name("key")?;
    let value = last.child_by_field_name("value").unwrap_or(key);
    if node_bytes(source, key) != node_bytes(source, value) {
        return None;
    }
    let call = dispatch(pair)?;
    if crate::cop::shared::is_setter_call(call)
        || parenthesized(source, call)
        || call
            .parent()
            .is_some_and(|p| p.kind() == "parenthesized_statements")
    {
        return None;
    }
    let as_argument = call
        .parent()
        .is_some_and(|p| matches!(p.kind(), "argument_list" | "call" | "super" | "yield"));
    if call.child_by_field_name("block").is_none() && last_expression(call) && !as_argument {
        return None;
    }
    Some(call)
}

pub(super) fn check(
    source: &SourceFile,
    pair: Node<'_>,
    config: &CopConfig,
    diagnostics: &mut Vec<Diagnostic>,
    mut corrections: Option<&mut Vec<Correction>>,
) {
    if config.get_f64("TargetRubyVersion", 2.7) <= 3.0 {
        return;
    }
    let style = config.get_str("EnforcedShorthandSyntax", "always");
    if style == "either" {
        return;
    }
    let Some(parent) = pair.parent() else {
        return;
    };
    if !matches!(
        parent.kind(),
        "hash" | "argument_list" | "element_reference" | "array"
    ) {
        return;
    }
    let Some(key) = pair.child_by_field_name("key") else {
        return;
    };
    let omitted = pair.child_by_field_name("value").is_none();
    let eligible = omittable(source, pair);
    let (omit, mixed) = match style {
        "always" if !omitted && eligible => (true, false),
        "never" if omitted => (false, false),
        "consistent" | "either_consistent" => {
            let siblings = siblings(source, pair);
            let has_omitted = siblings.has_omitted;
            let has_needed = siblings.has_needed;
            if has_needed && omitted {
                (false, true)
            } else if !has_needed && !omitted && eligible && (has_omitted || style == "consistent")
            {
                (true, has_omitted)
            } else {
                return;
            }
        }
        _ => return,
    };
    let action = if omit {
        "Omit the hash value."
    } else {
        "Include the hash value."
    };
    let message = if mixed {
        format!("Do not mix explicit and implicit hash values. {action}")
    } else {
        action.into()
    };
    let value = pair.child_by_field_name("value").unwrap_or(key);
    let (line, column) = source.offset_to_line_col(value.start_byte());
    let mut diagnostic = HashSyntax.diagnostic(source, line, column, message);
    diagnostic.highlight_length = String::from_utf8_lossy(node_bytes(source, value))
        .chars()
        .count();
    let key_text = String::from_utf8_lossy(node_bytes(source, key));
    let replacement = if omit {
        format!("{key_text}:")
    } else {
        format!("{key_text}: {key_text}")
    };
    let wrapper = wrap_call(source, pair);
    let wrapper_complete = wrapper.is_none()
        || Some(last_pair(source, pair)).is_some_and(|last| {
            if omit {
                last.child_by_field_name("value").is_some() && omittable(source, last)
            } else {
                last.child_by_field_name("value").is_none()
            }
        });
    if wrapper_complete
        && collect_comments(pair).is_empty()
        && operator(pair).is_some_and(|op| op.kind() == ":")
    {
        diagnostic.corrected = push_replace(
            &mut corrections,
            pair.start_byte(),
            pair.end_byte(),
            replacement,
            HashSyntax.name(),
        );
        if let Some(call) = wrapper {
            if parent.kind() == "argument_list" {
                let args = named_kids(parent);
                if let (Some(selector), Some(first), Some(last)) = (
                    call.child_by_field_name("method").or_else(|| call.child(0)),
                    args.first(),
                    args.last(),
                ) {
                    if last_pair(source, pair) == pair
                        && matches!(last.kind(), "pair" | "hash_splat_argument")
                    {
                        push_replace(
                            &mut corrections,
                            selector.end_byte(),
                            first.start_byte(),
                            "(",
                            HashSyntax.name(),
                        );
                        push_replace(
                            &mut corrections,
                            last.end_byte(),
                            last.end_byte(),
                            ")",
                            HashSyntax.name(),
                        );
                    }
                }
            }
        }
    } else {
        diagnostic.correctable = false;
    }
    diagnostics.push(diagnostic);
}
