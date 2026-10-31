//! Indent mismatch detection for multiline binary ops.

use tree_sitter::Node;

use crate::cop::CopConfig;
use crate::cop::layout::indentation_consistency_util as util;
use crate::cop::shared;
use crate::parse::source::SourceFile;

use super::context::{
    AssignmentContext, argument_in_method_call, assignment_context, assignment_from_ancestors,
    keyword_context,
};
use super::line_scan::KeywordContext;

fn node_end_line(source: &SourceFile, node: Node<'_>) -> usize {
    source
        .offset_to_line_col(node.end_byte().saturating_sub(1))
        .0
}

/// RuboCop skips ops inside `(...)` groups and parenthesized argument lists.
pub(super) fn not_for_this_cop(node: Node<'_>) -> bool {
    let mut p = node.parent();
    while let Some(n) = p {
        if matches!(n.kind(), "parenthesized_statements" | "interpolation") {
            return true;
        }
        if n.kind() == "argument_list" && arg_list_has_paren(n) {
            return true;
        }
        p = n.parent();
    }
    false
}

fn arg_list_has_paren(n: Node<'_>) -> bool {
    let mut cur = n.walk();
    n.children(&mut cur)
        .any(|c| !c.is_named() && c.kind() == "(")
}

fn multiline_rhs_candidate(source: &SourceFile, left: Node<'_>, right: Node<'_>) -> bool {
    node_end_line(source, left) != shared::node_line(source, right)
        && util::begins_its_line(source, right.start_byte())
        && shared::node_col(source, right) == shared::line_indent(source, right.start_byte())
}

fn keyword_extra(width: usize, keyword_ctx: Option<&KeywordContext>) -> usize {
    keyword_ctx
        .filter(|c| c.special_indent)
        .map(|_| width)
        .unwrap_or(0)
}

fn indented_anchor_col(
    source: &SourceFile,
    left: Node<'_>,
    width: usize,
    keyword_ctx: Option<&KeywordContext>,
) -> usize {
    shared::line_indent(source, left.start_byte()) + width + keyword_extra(width, keyword_ctx)
}

fn should_align_op(
    style: &str,
    keyword_ctx: Option<KeywordContext>,
    assignment_ctx: Option<AssignmentContext>,
    in_call_arg: bool,
) -> bool {
    assignment_ctx.is_some_and(|c| c.rhs_begins_line)
        || (style == "aligned"
            && (keyword_ctx.is_some() || assignment_ctx.is_some() || in_call_arg))
}

fn operation_expected_col(
    source: &SourceFile,
    node: Node<'_>,
    left: Node<'_>,
    width: usize,
    style: &str,
) -> (usize, bool) {
    let left_line = shared::node_line(source, left);
    let left_col = shared::node_col(source, left);
    let keyword_ctx = keyword_context(source, node, left_line, left_col);
    if should_align_op(
        style,
        keyword_ctx,
        assignment_from_ancestors(source, node)
            .or_else(|| assignment_context(source, left_line, left_col)),
        argument_in_method_call(node),
    ) {
        (left_col, true)
    } else {
        (
            indented_anchor_col(source, left, width, keyword_ctx.as_ref()),
            false,
        )
    }
}

pub(super) fn indent_mismatch(
    source: &SourceFile,
    node: Node<'_>,
    config: &CopConfig,
) -> Option<(usize, usize, bool)> {
    let width = config.get_usize("IndentationWidth", 2);
    let style = config.get_str("EnforcedStyle", "aligned");
    let left = node.child_by_field_name("left")?;
    let right = node.child_by_field_name("right")?;
    if !multiline_rhs_candidate(source, left, right) {
        return None;
    }
    let actual = shared::line_indent(source, right.start_byte());
    let (expected, align_only) = operation_expected_col(source, node, left, width, style);
    (actual != expected).then_some((actual, expected, align_only))
}

/// Expected column for the method-name part of a multiline dotted call (`aligned` style).
pub(crate) fn aligned_method_call_col(
    source: &SourceFile,
    call: Node<'_>,
    receiver: Node<'_>,
    width: usize,
) -> usize {
    let left_line = shared::node_line(source, receiver);
    let left_col = shared::node_col(source, receiver);
    let keyword_ctx = keyword_context(source, call, left_line, left_col);
    // `payload = {\n  key: (expr)\n    .new` aligns `.new` with `{`, not `(`.
    if keyword_ctx.is_none() {
        if let Some(col) = assignment_rhs_col(source, call) {
            return col;
        }
    }
    let assignment_ctx = assignment_from_ancestors(source, call)
        .or_else(|| assignment_context(source, left_line, left_col));
    let should_align = assignment_ctx.is_some_and(|c| c.rhs_begins_line)
        || keyword_ctx.is_some()
        || assignment_ctx.is_some();
    if should_align {
        left_col
    } else {
        indented_anchor_col(source, receiver, width, keyword_ctx.as_ref())
    }
}

fn contains_node(inner: Node<'_>, outer: Node<'_>) -> bool {
    inner.start_byte() >= outer.start_byte() && inner.end_byte() <= outer.end_byte()
}

/// RuboCop stops syntactic assignment alignment at these ancestors.
fn blocks_rhs_align(node: Node<'_>, anc: Node<'_>) -> bool {
    matches!(
        anc.kind(),
        "if" | "while" | "until" | "for" | "return" | "array" | "begin"
    ) || in_block_body(node, anc)
}

fn in_block_body(node: Node<'_>, anc: Node<'_>) -> bool {
    if !matches!(anc.kind(), "block" | "do_block") {
        return false;
    }
    anc.child_by_field_name("body")
        .is_some_and(|body| contains_node(node, body))
}

fn assigned_rhs_col(source: &SourceFile, node: Node<'_>, anc: Node<'_>) -> Option<usize> {
    if !matches!(anc.kind(), "assignment" | "operator_assignment") {
        return None;
    }
    let rhs = anc.child_by_field_name("right")?;
    contains_node(node, rhs).then(|| source.offset_to_line_col(rhs.start_byte()).1)
}

/// Column of the assignment RHS when this call sits inside it.
fn assignment_rhs_col(source: &SourceFile, node: Node<'_>) -> Option<usize> {
    let mut p = node.parent();
    while let Some(anc) = p {
        if blocks_rhs_align(node, anc) {
            return None;
        }
        if let Some(col) = assigned_rhs_col(source, node, anc) {
            return Some(col);
        }
        p = anc.parent();
    }
    None
}
