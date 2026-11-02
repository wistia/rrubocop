use crate::cop::rspec::helpers::{RSPEC_INCLUDE, call_block};
use crate::cop::rspec::parity_helpers::*;
use crate::cop::rspec::separation::numbered_block;
use crate::cop::shared::{call_receiver, node_bytes};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;
use tree_sitter::Node;

fn change(source: &SourceFile, node: Node<'_>) -> bool {
    call_receiver(node).is_none()
        && !safe_send(source, node)
        && matches!(
            name(source, node),
            Some(b"change" | b"a_block_changing" | b"changing")
        )
}
fn getter<'a>(source: &SourceFile, node: Node<'a>) -> Option<(Node<'a>, String)> {
    if let Some(block) = call_block(node) {
        if !call_args(node).is_empty()
            || numbered_block(source, node)
            || block
                .child_by_field_name("parameters")
                .is_some_and(|p| !code_children(p).is_empty())
        {
            return None;
        }
        let body = block
            .child_by_field_name("body")
            .map(code_children)
            .unwrap_or_default();
        if body.len() != 1
            || body[0].kind() != "call"
            || safe_send(source, body[0])
            || !call_args(body[0]).is_empty()
            || call_block(body[0]).is_some()
        {
            return None;
        }
        Some((
            call_receiver(body[0])?,
            String::from_utf8_lossy(name(source, body[0])?).into_owned(),
        ))
    } else {
        let args = call_args(node);
        if args.len() != 2 {
            return None;
        }
        Some((
            args[0],
            crate::cop::ported::static_literal(source, args[1])?,
        ))
    }
}
fn stable_numeric_getter(source: &SourceFile, node: Node<'_>) -> bool {
    let Some((receiver, message)) = getter(source, node) else {
        return false;
    };
    matches!(message.as_str(), "size" | "length" | "count")
        && matches!(receiver.kind(), "array" | "hash" | "string" | "character")
        && pure_literal(source, receiver)
        || message == "itself" && integer_value(source, receiver).is_some()
}
fn block_matches(source: &SourceFile, node: Node<'_>) -> bool {
    let Some(block) = call_block(node) else {
        return true;
    };
    if !call_args(node).is_empty()
        || numbered_block(source, node)
        || block
            .child_by_field_name("parameters")
            .is_some_and(|p| !code_children(p).is_empty())
    {
        return false;
    }
    let body = block
        .child_by_field_name("body")
        .map(code_children)
        .unwrap_or_default();
    body.len() == 1
        && (body[0].kind() == "call"
            && !safe_send(source, body[0])
            && call_receiver(body[0]).is_some()
            && call_args(body[0]).is_empty()
            && call_block(body[0]).is_none()
            || body[0].kind() == "unary"
                && integer_value(source, body[0]).is_none()
                && body[0]
                    .child_by_field_name("operand")
                    .is_none_or(|operand| {
                        !matches!(operand.kind(), "float" | "rational" | "complex")
                    })
            || body[0].kind() == "element_reference"
                && code_children(body[0])
                    .iter()
                    .all(|child| Some(*child) == body[0].child_by_field_name("object")))
}
fn compound(source: &SourceFile, node: Node<'_>) -> bool {
    matches!(name(source, node), Some(b"and" | b"or" | b"&" | b"|"))
        || node.kind() == "binary"
            && node
                .child_by_field_name("operator")
                .is_some_and(|op| matches!(node_bytes(source, op), b"&" | b"|"))
}

pub struct ChangeByZero;
impl Cop for ChangeByZero {
    fn name(&self) -> &'static str {
        "RSpec/ChangeByZero"
    }
    fn default_include(&self) -> &'static [&'static str] {
        RSPEC_INCLUDE
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call"]
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if name(source, node) != Some(b"by") || safe_send(source, node) {
            return;
        }
        let args = call_args(node);
        if args.len() != 1 || integer_value(source, args[0]).is_none_or(|n| n != 0.into()) {
            return;
        }
        let Some(target) =
            call_receiver(node).filter(|n| change(source, *n) && block_matches(source, *n))
        else {
            return;
        };
        let Some(parent) = node.parent().and_then(|parent| {
            if parent.kind() == "argument_list" {
                parent.parent()
            } else {
                Some(parent)
            }
        }) else {
            return;
        };
        let is_compound = compound(source, parent);
        let method = String::from_utf8_lossy(name(source, target).unwrap());
        let negated = config.get_str("NegatedMatcher", "");
        let message = if is_compound {
            let preferred = if negated.is_empty() {
                "negated matchers".into()
            } else {
                format!("`{negated}`")
            };
            format!("Prefer {preferred} with compound expectations over `{method}.by(0)`.")
        } else {
            format!("Prefer `not_to change` over `to {method}.by(0)`.")
        };
        let index = report(
            self,
            source,
            node.start_byte(),
            node.end_byte(),
            message,
            diagnostics,
        );
        if is_compound && negated.is_empty() {
            diagnostics[index].correctable = false;
        }
        if corrections.is_none()
            || !stable_numeric_getter(source, target)
            || is_compound && negated.is_empty()
        {
            return;
        }
        let replacement_name = if is_compound { negated } else { "change" };
        if !replacement_name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
            || replacement_name.is_empty()
        {
            return;
        }
        let mut replacement = node_bytes(source, target).to_vec();
        let method = selector(target);
        replacement.splice(
            method.start_byte() - target.start_byte()..method.end_byte() - target.start_byte(),
            replacement_name.bytes(),
        );
        if !is_compound {
            if name(source, parent) != Some(b"to") || !runner(source, parent) {
                return;
            }
            let start = selector(parent).start_byte();
            let end = node.end_byte();
            let mut rewritten = source.as_bytes()[start..end].to_vec();
            rewritten.splice(
                node.start_byte() - start..node.end_byte() - start,
                replacement,
            );
            rewritten.splice(
                0..selector(parent).end_byte() - start,
                b"not_to".iter().copied(),
            );
            if preserving_replace(
                source,
                node,
                &mut corrections,
                start,
                end,
                String::from_utf8_lossy(&rewritten),
                self.name(),
            ) {
                diagnostics[index].corrected = true;
            }
        } else if preserving_replace(
            source,
            node,
            &mut corrections,
            node.start_byte(),
            node.end_byte(),
            String::from_utf8_lossy(&replacement),
            self.name(),
        ) {
            diagnostics[index].corrected = true;
        }
    }
}
