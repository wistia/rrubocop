//! Layout/ConditionPosition.

use tree_sitter::Node;

use crate::cop::shared;
use crate::cop::{Cop, CopConfig};
use crate::correction::Correction;
use crate::diagnostic::Diagnostic;
use crate::parse::source::SourceFile;

pub struct ConditionPosition;

fn keyword_end(node: Node<'_>) -> usize {
    let mut cur = node.walk();
    for c in node.children(&mut cur) {
        if !c.is_named() && matches!(c.kind(), "if" | "unless" | "while" | "until" | "elsif") {
            return c.end_byte();
        }
    }
    node.start_byte() + node.kind().len()
}

fn same_line_body_range(
    source: &SourceFile,
    node: Node<'_>,
    cond: Node<'_>,
) -> Option<(usize, usize)> {
    let body = node
        .child_by_field_name("consequence")
        .or_else(|| node.child_by_field_name("body"))?;
    let (body_line, _) = source.offset_to_line_col(body.start_byte());
    let (cond_last, _) = source.offset_to_line_col(cond.end_byte().saturating_sub(1));
    if body_line == cond_last {
        Some((cond.start_byte(), body.start_byte()))
    } else {
        None
    }
}

fn full_line_range(source: &SourceFile, cond: Node<'_>) -> (usize, usize) {
    let bytes = source.as_bytes();
    let (line, _) = source.offset_to_line_col(cond.start_byte());
    let start = source.line_start(line).unwrap_or(cond.start_byte());
    let mut end = cond.end_byte();
    while end < bytes.len() && bytes[end] != b'\n' {
        end += 1;
    }
    if end < bytes.len() && bytes[end] == b'\n' {
        end += 1;
    }
    (start, end)
}

fn removal_range(source: &SourceFile, node: Node<'_>, cond: Node<'_>) -> (usize, usize) {
    same_line_body_range(source, node, cond).unwrap_or_else(|| full_line_range(source, cond))
}

fn keyword_of(kind: &str) -> Option<&'static str> {
    match kind {
        "if" => Some("if"),
        "unless" => Some("unless"),
        "while" => Some("while"),
        "until" => Some("until"),
        "elsif" => Some("elsif"),
        _ => None,
    }
}

fn should_check(node: Node<'_>) -> bool {
    if node.kind() == "if" && shared::child_kind(node, "?").is_some() {
        return false;
    }
    shared::end_keyword(node).is_some() || node.kind() == "elsif"
}

fn apply_move(
    cop: &dyn Cop,
    source: &SourceFile,
    node: Node<'_>,
    cond: Node<'_>,
    corr: &mut Vec<Correction>,
) -> bool {
    let cond_src = shared::node_text(source, cond);
    let kw_end = keyword_end(node);
    let (rm_start, rm_end) = removal_range(source, node, cond);
    corr.push(Correction {
        start: kw_end,
        end: kw_end,
        replacement: format!(" {cond_src}"),
        cop_name: cop.name(),
        cop_index: 0,
        owner: None,
    });
    if rm_start < kw_end {
        return false;
    }
    corr.push(Correction {
        start: rm_start,
        end: rm_end,
        replacement: String::new(),
        cop_name: cop.name(),
        cop_index: 1,
        owner: None,
    });
    true
}

impl Cop for ConditionPosition {
    fn name(&self) -> &'static str {
        "Layout/ConditionPosition"
    }
    fn supports_autocorrect(&self) -> bool {
        true
    }
    fn interested_node_kinds(&self) -> &'static [&'static str] {
        &["if", "unless", "while", "until", "elsif"]
    }

    fn check_node(
        &self,
        source: &SourceFile,
        node: Node<'_>,
        _config: &CopConfig,
        diagnostics: &mut Vec<Diagnostic>,
        mut corrections: Option<&mut Vec<Correction>>,
    ) {
        if !should_check(node) {
            return;
        }
        let Some(keyword) = keyword_of(node.kind()) else {
            return;
        };
        let Some(cond) = node.child_by_field_name("condition") else {
            return;
        };
        if shared::node_line(source, node) == source.offset_to_line_col(cond.start_byte()).0 {
            return;
        }
        let (pred_line, pred_col) = source.offset_to_line_col(cond.start_byte());
        let mut diag = self.diagnostic(
            source,
            pred_line,
            pred_col,
            format!("Place the condition on the same line as `{keyword}`."),
        );
        if let Some(corr) = corrections.as_mut() {
            if apply_move(self, source, node, cond, corr) {
                diag.corrected = true;
            }
        }
        diagnostics.push(diag);
    }
}
