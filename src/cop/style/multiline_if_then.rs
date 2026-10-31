//! Style/MultilineIfThen — no `then` when body starts on the next line.

use tree_sitter::Node;

use crate::cop::shared::{node_bytes, node_line};
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct MultilineIfThen;

impl Cop for MultilineIfThen {
    fn name(&self) -> &'static str {
        "Style/MultilineIfThen"
    }

    fn supports_autocorrect(&self) -> bool {
        true
    }

    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["if", "unless", "elsif"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if node.start_position().row == node.end_position().row {
            return;
        }
        let Some(then_kw) = find_then_keyword(source, node) else {
            return;
        };
        // RuboCop: allow `if cond then a` / `elsif cond then b` when body shares the then line.
        if then_body_same_line(source, node, then_kw) {
            return;
        }
        report(self, source, node, then_kw, diagnostics, &mut corrections);
    }
}

fn keyword_label(kind: &str) -> &'static str {
    match kind {
        "unless" => "unless",
        "elsif" => "elsif",
        _ => "if",
    }
}

fn report(
    cop: &MultilineIfThen,
    source: &SourceFile,
    node: Node<'_>,
    then_kw: Node<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    corrections: &mut Option<&mut Vec<Correction>>,
) {
    let kw = keyword_label(node.kind());
    let (line, col) = source.offset_to_line_col(then_kw.start_byte());
    let mut diag = cop.diagnostic(
        source,
        line,
        col,
        format!("Do not use `then` for multi-line `{kw}`."),
    );
    if let Some(corr) = corrections {
        let src = source.as_bytes();
        let mut remove_start = then_kw.start_byte();
        while remove_start > 0 && src[remove_start - 1] == b' ' {
            remove_start -= 1;
        }
        corr.push(Correction {
            start: remove_start,
            end: then_kw.end_byte(),
            replacement: String::new(),
            cop_name: cop.name(),
            cop_index: 0,
            owner: None,
        });
        diag.corrected = true;
    }
    diagnostics.push(diag);
}

fn then_body_same_line(source: &SourceFile, node: Node<'_>, then_kw: Node<'_>) -> bool {
    let then_line = node_line(source, then_kw);
    if let Some(body) = consequence_body(node) {
        return node_line(source, body) == then_line;
    }
    // Empty body after `then` — still same line if nothing follows on a later line
    // before `end`/`elsif`/`else` on this branch; RuboCop flags `then` alone on a line.
    false
}

fn consequence_body(node: Node<'_>) -> Option<Node<'_>> {
    if let Some(c) = node.child_by_field_name("consequence") {
        return first_stmt_in(c);
    }
    let mut cur = node.walk();
    for child in node.named_children(&mut cur) {
        if child.kind() == "then" {
            return first_stmt_in(child);
        }
    }
    None
}

fn first_stmt_in(node: Node<'_>) -> Option<Node<'_>> {
    let mut cur = node.walk();
    node.named_children(&mut cur)
        .find(|n| n.kind() != "comment")
}

fn find_then_keyword<'a>(source: &SourceFile, node: Node<'a>) -> Option<Node<'a>> {
    let mut cur = node.walk();
    for child in node.children(&mut cur) {
        if node_bytes(source, child) == b"then" {
            return Some(child);
        }
        if let Some(kw) = then_in_then_node(source, child) {
            return Some(kw);
        }
    }
    None
}

fn then_in_then_node<'a>(source: &SourceFile, child: Node<'a>) -> Option<Node<'a>> {
    if child.kind() != "then" {
        return None;
    }
    let mut tc = child.walk();
    child
        .children(&mut tc)
        .find(|gc| node_bytes(source, *gc) == b"then")
}

#[cfg(test)]
mod tests {
    use super::*;
    crate::cop_fixture_tests!(MultilineIfThen, "cops/style/multiline_if_then");
}
