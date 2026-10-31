//! Layout/EmptyLinesAroundAccessModifier.

use tree_sitter::{Node, Tree};

use crate::cop::shared;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::codemap::CodeMap;
use crate::parse::source::SourceFile;

pub struct EmptyLinesAroundAccessModifier;

fn modifier_name<'a>(source: &'a SourceFile, n: Node<'_>) -> Option<&'a [u8]> {
    if n.kind() == "identifier" {
        Some(shared::node_bytes(source, n))
    } else {
        shared::call_method_name(source, n)
    }
}

fn is_modifier(name: &[u8]) -> bool {
    matches!(
        name,
        b"private" | b"protected" | b"public" | b"module_function"
    )
}

fn bare_modifier(n: Node<'_>) -> bool {
    if n.kind() == "identifier" {
        return !matches!(
            n.parent().map(|p| p.kind()),
            Some("call" | "command" | "command_call")
        );
    }
    // `obj.private` is a method call, not an access modifier.
    if crate::cop::shared::call_receiver(n).is_some() {
        return false;
    }
    match n.child_by_field_name("arguments") {
        Some(a) => a.named_child_count() == 0,
        None => true,
    }
}

fn is_comment_line(source: &SourceFile, line: usize) -> bool {
    let Some(start) = source.line_start(line) else {
        return false;
    };
    let bytes = source.as_bytes();
    let mut i = start;
    while i < bytes.len() && bytes[i] != b'\n' {
        match bytes[i] {
            b' ' | b'\t' | b'\r' => i += 1,
            b'#' => return true,
            _ => return false,
        }
    }
    false
}

fn superclass_or_self(class_node: Node<'_>) -> Option<Node<'_>> {
    class_node
        .child_by_field_name("superclass")
        .or_else(|| class_node.child_by_field_name("value"))
}

/// RuboCop `class_def?` / `block_start?`: modifier on the line right after the body opener.
fn at_body_opening(source: &SourceFile, n: Node<'_>) -> bool {
    let line = shared::node_line(source, n);
    let mut cur = n.parent();
    while let Some(parent) = cur {
        match parent.kind() {
            "body_statement" | "block_body" => cur = parent.parent(),
            "class" | "module" | "singleton_class" => {
                return line
                    == superclass_or_self(parent)
                        .map(|s| shared::node_line(source, s))
                        .unwrap_or_else(|| shared::node_line(source, parent))
                        + 1;
            }
            "do_block" | "block" | "lambda" => {
                return line == shared::node_line(source, parent) + 1;
            }
            _ => return false,
        }
    }
    false
}

fn previous_line_ok(source: &SourceFile, line: usize, n: Node<'_>) -> bool {
    if at_body_opening(source, n) || line <= 1 {
        return true;
    }
    let mut prev = line - 1;
    while prev >= 1 && is_comment_line(source, prev) {
        prev -= 1;
    }
    prev < 1 || shared::line_blank(source, prev)
}

fn in_method_body(n: Node<'_>) -> bool {
    let mut p = n.parent();
    while let Some(parent) = p {
        match parent.kind() {
            "method" | "singleton_method" => return true,
            "class" | "module" | "singleton_class" | "do_block" | "block" | "program" => {
                return false;
            }
            _ => p = parent.parent(),
        }
    }
    false
}

fn push_newline_at(
    corrections: &mut Option<&mut Vec<Correction>>,
    source: &SourceFile,
    line: usize,
    cop_name: &'static str,
) -> bool {
    let Some(corr) = corrections.as_deref_mut() else {
        return false;
    };
    let Some(offset) = source.line_start(line) else {
        return false;
    };
    corr.push(Correction {
        start: offset,
        end: offset,
        replacement: "\n".into(),
        cop_name,
        cop_index: 0,
        owner: None,
    });
    true
}

fn modifier_message(
    name: &[u8],
    style: &str,
    at_opening: bool,
    before_ok: bool,
    need_after: bool,
) -> String {
    let modifier = String::from_utf8_lossy(name);
    if style == "only_before" {
        format!("Keep a blank line before `{modifier}`.")
    } else if at_opening || (before_ok && need_after) {
        format!("Keep a blank line after `{modifier}`.")
    } else {
        format!("Keep a blank line before and after `{modifier}`.")
    }
}

fn blank_needs(style: &str, before_ok: bool, after_ok: bool) -> (bool, bool) {
    (
        (style == "around" || style == "only_before") && !before_ok,
        (style == "around" || style == "only_after") && !after_ok,
    )
}

fn access_modifier_at<'a>(source: &'a SourceFile, n: Node<'a>) -> Option<&'a [u8]> {
    if !matches!(n.kind(), "call" | "command" | "identifier") {
        return None;
    }
    let name = modifier_name(source, n)?;
    (is_modifier(name) && bare_modifier(n) && !in_method_body(n)).then_some(name)
}

fn emit_modifier(
    cop: &dyn Cop,
    source: &SourceFile,
    n: Node<'_>,
    name: &[u8],
    style: &str,
    line: usize,
    before_ok: bool,
    need_before: bool,
    need_after: bool,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let (diag_line, diag_col) = source.offset_to_line_col(n.start_byte());
    let mut diag = cop.diagnostic(
        source,
        diag_line,
        diag_col,
        modifier_message(
            name,
            style,
            at_body_opening(source, n),
            before_ok,
            need_after,
        ),
    );
    let mut fixed = false;
    if need_before {
        fixed |= push_newline_at(corrections, source, line, cop.name());
    }
    if need_after {
        fixed |= push_newline_at(corrections, source, line + 1, cop.name());
    }
    if fixed {
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

fn check_modifier(
    cop: &dyn Cop,
    source: &SourceFile,
    n: Node<'_>,
    style: &str,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let Some(name) = access_modifier_at(source, n) else {
        return;
    };
    let line = shared::node_line(source, n);
    if n.next_named_sibling().is_none() {
        return;
    }
    let before_ok = previous_line_ok(source, line, n);
    let (need_before, need_after) =
        blank_needs(style, before_ok, shared::line_blank(source, line + 1));
    if need_before || need_after {
        emit_modifier(
            cop,
            source,
            n,
            name,
            style,
            line,
            before_ok,
            need_before,
            need_after,
            diagnostics,
            corrections,
        );
    }
}

impl Cop for EmptyLinesAroundAccessModifier {
    fn name(&self) -> &'static str {
        "Layout/EmptyLinesAroundAccessModifier"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn uses_source_phase(&self) -> bool {
        true
    }

    fn check_source(
        &self,
        source: &SourceFile,
        tree: &Tree,
        code_map: &CodeMap,
        config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        let _ = code_map;
        let style = config.get_str("EnforcedStyle", "around");
        shared::for_each_descendant(tree.root_node(), |n| {
            check_modifier(self, source, n, style, diagnostics, &mut corrections);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(
        EmptyLinesAroundAccessModifier,
        "cops/layout/empty_lines_around_access_modifier"
    );
}
