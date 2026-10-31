//! Autocorrect: drop a yielding block and pass `&block` instead.

use tree_sitter::Node;

use crate::correction::Correction;
use crate::parse::source::SourceFile;

use super::edit::{is_zsuper, list_edit, zsuper_args};
use super::name::{block_param, pass_name};
use super::{COP, host};

pub(super) fn rewrite(
    source: &SourceFile,
    block: Node<'_>,
    corrections: &mut Option<&mut Vec<Correction>>,
) -> bool {
    let call = host(block);
    if !matches!(call.kind(), "call" | "command" | "command_call") {
        return false;
    }
    let Some(method) = def_node(block) else {
        return false;
    };
    let Some(name) = pass_name(source, method) else {
        return false;
    };
    add_def_block(source, method, corrections, &name);
    push_edit(
        corrections,
        call.start_byte(),
        call.end_byte(),
        call_replacement(source, call, method, &name),
    )
}

fn def_node(node: Node<'_>) -> Option<Node<'_>> {
    let mut p = node.parent();
    while let Some(n) = p {
        if matches!(n.kind(), "method" | "singleton_method") {
            return Some(n);
        }
        p = n.parent();
    }
    None
}

fn call_end_before_block(call: Node<'_>) -> usize {
    call.child_by_field_name("arguments")
        .or_else(|| call.child_by_field_name("method"))
        .map(|n| n.end_byte())
        .unwrap_or(call.start_byte())
}

fn spliced_call(source: &SourceFile, call: Node<'_>, name: &str) -> String {
    let base = call.start_byte();
    let mut head = source.as_bytes()[base..call_end_before_block(call)].to_vec();
    if let Some(args) = call.child_by_field_name("arguments") {
        if let Some((start, end, repl)) = list_edit(source, args, name) {
            head.splice(start - base..end - base, repl.into_bytes());
        }
    } else {
        head.extend(format!("(&{name})").into_bytes());
    }
    String::from_utf8_lossy(&head).into_owned()
}

fn call_replacement(source: &SourceFile, call: Node<'_>, method: Node<'_>, name: &str) -> String {
    if is_zsuper(call) {
        format!("super({})", zsuper_args(source, method, name))
    } else {
        spliced_call(source, call, name)
    }
}

fn push_edit(
    corrections: &mut Option<&mut Vec<Correction>>,
    start: usize,
    end: usize,
    replacement: String,
) -> bool {
    let Some(buf) = corrections.as_deref_mut() else {
        return false;
    };
    if start == end
        && buf
            .iter()
            .any(|c| c.cop_name == COP && c.start == start && c.end == end)
    {
        return true;
    }
    buf.push(Correction {
        start,
        end,
        replacement,
        cop_name: COP,
        cop_index: 0,
        owner: None,
    });
    true
}

fn add_def_block(
    source: &SourceFile,
    method: Node<'_>,
    corrections: &mut Option<&mut Vec<Correction>>,
    name: &str,
) {
    let Some(params) = method.child_by_field_name("parameters") else {
        let Some(method_name) = method.child_by_field_name("name") else {
            return;
        };
        push_edit(
            corrections,
            method_name.end_byte(),
            method_name.end_byte(),
            format!("(&{name})"),
        );
        return;
    };
    if block_param(source, params).is_some() {
        return;
    }
    if let Some((start, end, text)) = list_edit(source, params, name) {
        push_edit(corrections, start, end, text);
    }
}
