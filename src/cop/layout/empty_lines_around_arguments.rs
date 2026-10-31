//! Layout/EmptyLinesAroundArguments.

use tree_sitter::Node;

use crate::cop::shared;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct EmptyLinesAroundArguments;

fn node_end_line(source: &SourceFile, node: Node<'_>) -> usize {
    source
        .offset_to_line_col(node.end_byte().saturating_sub(1))
        .0
}

fn receiver_method_same_line(source: &SourceFile, call: Node<'_>) -> bool {
    let Some(recv) = call.child_by_field_name("receiver") else {
        return true;
    };
    let Some(meth) = call
        .child_by_field_name("method")
        .or_else(|| call.child_by_field_name("name"))
    else {
        return true;
    };
    node_end_line(source, recv) == shared::node_line(source, meth)
}

fn arg_nodes(args: Node<'_>) -> Vec<Node<'_>> {
    let mut cur = args.walk();
    args.named_children(&mut cur)
        .filter(|n| !matches!(n.kind(), "comment" | "heredoc_body" | "heredoc_end"))
        .collect()
}

/// RuboCop treats keyword args in parens as one argument; tree-sitter lists each pair separately.
fn top_level_args(args: Node<'_>) -> Vec<Node<'_>> {
    let mut nodes = arg_nodes(args);
    if let Some(first_pair) = nodes.iter().position(|n| n.kind() == "pair") {
        nodes.truncate(first_pair + 1);
    }
    nodes
}

fn line_blank(source: &SourceFile, line: usize) -> bool {
    let Some(start) = source.line_start(line) else {
        return true;
    };
    let bytes = source.as_bytes();
    let mut i = start;
    while i < bytes.len() && bytes[i] != b'\n' {
        if bytes[i] != b' ' && bytes[i] != b'\t' && bytes[i] != b'\r' {
            return false;
        }
        i += 1;
    }
    true
}

fn remove_blank(
    cop: &dyn Cop,
    source: &SourceFile,
    line: usize,
    at: usize,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (l, c) = source.offset_to_line_col(at);
    let mut diag = cop.diagnostic(source, l, c, "Empty line detected around arguments.".into());
    if let Some(corr) = corrections {
        if let Some(s) = source.line_start(line) {
            corr.push(Correction {
                start: s,
                end: source.line_start(line + 1).unwrap_or(s),
                replacement: String::new(),
                cop_name: cop.name(),
                cop_index: 0,
                owner: None,
            });
            diag.corrected = true;
        }
    }
    diagnostics.push(diag);
}

fn check_blank_before(
    cop: &dyn Cop,
    source: &SourceFile,
    offset: usize,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let bytes = source.as_bytes();
    let (target_line, _) = source.offset_to_line_col(offset);
    let mut pos = offset;
    while pos > 0 {
        match bytes[pos - 1] {
            b' ' | b'\t' | b'\r' | b'\n' => pos -= 1,
            _ => break,
        }
    }
    let prev_line = if pos == 0 {
        1
    } else {
        source.offset_to_line_col(pos.saturating_sub(1)).0
    };
    if target_line > prev_line + 1 {
        let line_num = target_line - 1;
        if line_blank(source, line_num) {
            remove_blank(cop, source, line_num, offset, diagnostics, corrections);
        }
    }
}

fn closing_paren_offset(source: &SourceFile, call: Node<'_>) -> Option<usize> {
    let args = call.child_by_field_name("arguments")?;
    let bytes = source.as_bytes();
    let close = args.end_byte().saturating_sub(1);
    (bytes.get(close) == Some(&b')')).then_some(close)
}

fn check_call(
    cop: &dyn Cop,
    source: &SourceFile,
    call: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let args = match call.child_by_field_name("arguments") {
        Some(a) => a,
        None => return,
    };
    let items = top_level_args(args);
    if items.is_empty() {
        return;
    }
    if shared::node_line(source, call) == node_end_line(source, call) {
        return;
    }
    if !receiver_method_same_line(source, call) {
        return;
    }
    for arg in items {
        check_blank_before(cop, source, arg.start_byte(), diagnostics, corrections);
    }
    if let Some(close) = closing_paren_offset(source, call) {
        check_blank_before(cop, source, close, diagnostics, corrections);
    }
}

impl Cop for EmptyLinesAroundArguments {
    fn name(&self) -> &'static str {
        "Layout/EmptyLinesAroundArguments"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["call", "command"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let _ = config;
        check_call(self, source, node, diagnostics, &mut corrections);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    crate::cop_fixture_tests!(
        EmptyLinesAroundArguments,
        "cops/layout/empty_lines_around_arguments"
    );
}
